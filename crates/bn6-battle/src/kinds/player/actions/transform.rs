//! Action 0x1C as a form change (`sub_8014A38`): while the battle is paused
//! at the start of a turn, the navi changes into the form its side asked
//! for (`Battle::turn_transforms`). The transformation sequencer waits for
//! it. Crosses and Beast Out are implemented. See docs/engine/battle-flow.md
//! §3.4.1 and objects-and-player.md §12.9-§12.10.

use crate::actor::{request, status};
use crate::battle::Battle;
use crate::collision::{f1, link, timer};
use crate::data::player as pdata;
use super::ActionVars;
use crate::kinds::common;
use crate::kinds::player::status::end_anger;
use crate::kinds::player::{
    ai, ai_mut, clear_flag1, clear_flag2, clear_invulnerable, coll_mut, emotion, exit_attack_state, form,
    reset_charge, reset_status, set_coordinates_from_panel, set_mood, snap_to_future_panel, stats, stats_mut,
};
use crate::kinds::{cross_merge, effect, form_overlay, palette_flash};
use crate::object::{ObjectRef, flags};
use crate::setup::{Form, Navi};

/// The steps of Beast Out (`off_8014AC8`), in the attack step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BeastOutStep {
    /// Stop everything and strike the pose.
    Prepare = 0,
    /// Wait, then vanish into the beast effect.
    Vanish = 4,
    /// Come back in the new form.
    Emerge = 8,
    /// A short pause, then back to idle.
    Settle = 0xC,
}

impl BeastOutStep {
    fn of(b: &Battle, r: ObjectRef) -> BeastOutStep {
        match ai(b, r).attack.step {
            0 => BeastOutStep::Prepare,
            4 => BeastOutStep::Vanish,
            8 => BeastOutStep::Emerge,
            0xC => BeastOutStep::Settle,
            s => panic!("Beast Out step {s:#x}"),
        }
    }
}

/// The form change's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vars {
    /// Ticks left in the current step.
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

fn set_step(b: &mut Battle, r: ObjectRef, step: BeastOutStep) {
    let a = &mut ai_mut(b, r).attack;
    a.step = step as u8;
    a.step_init = 0;
}

/// Count the timer down; true once it had run out (it wraps).
fn timer_ran_out(b: &mut Battle, r: ObjectRef) -> bool {
    let v = vars(b, r);
    let old = v.timer;
    v.timer = old.wrapping_sub(1);
    old == 0
}

/// `sub_8014A38`: one tick of the form change.
pub(in crate::kinds::player) fn form_change(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    let Some(target) = b.turn_transforms[side].form.filter(|f| (1..=0x18).contains(&f.0)) else {
        ai_mut(b, r).status &= !status::FORM_CHANGE;
        return;
    };
    let current = stats(b, r).form;
    match target.0 {
        0x17..=0x18 => panic!("Beast Over (sub_801516C) is not implemented yet"),
        0x0D..=0x16 if current.0 > 0x0A => panic!("Cross Beast form changes (sub_80153EC) are not implemented yet"),
        0x0D..=0x16 => panic!("Cross Beast (sub_8014F40) is not implemented yet"),
        0x0B..=0x0C => beast_out(b, r, target),
        _ => cross(b, r, target),
    }
    if ai(b, r).status & status::FORM_CHANGE_SPRITE_HELD == 0 {
        common::step_sprite(b, r);
    }
}

fn beast_out(b: &mut Battle, r: ObjectRef, target: Form) {
    match BeastOutStep::of(b, r) {
        BeastOutStep::Prepare => prepare(b, r),
        BeastOutStep::Vanish => vanish(b, r),
        BeastOutStep::Emerge => emerge(b, r, target),
        BeastOutStep::Settle => settle(b, r),
    }
}

/// `sub_8014D08`: onto the destination panel, facing the default way,
/// charge and statuses dropped, links cut; the Beast Out pose.
fn prepare(b: &mut Battle, r: ObjectRef) {
    snap_to_future_panel(b, r);
    clear_invulnerable(b, r);
    // sub_800F46C: the standard column patterns face the default way.
    if matches!(b.setup.settings.panel_pattern, 0x38 | 0x30 | 0x3C) {
        b.objects.get_mut(r).flip = 0;
    }
    reset_charge(b, r);
    // sub_80C4C3A: the Full Synchro aura goes.
    if ai(b, r).full_synchro_aura.is_some() {
        panic!("ending the Full Synchro aura (sub_80C4C3A) is not implemented yet");
    }
    b.objects.get_mut(r).related[0] = None;
    ai_mut(b, r).overlay = None;
    drop_statuses(b, r);
    b.objects.get_mut(r).anim = 0x11;
    if b.objects.get(r).related[1].is_some() {
        // Sets the overlay's Param3 to 1 and flags 0x14.
        panic!("changing form with a form overlay on is not implemented yet");
    }
    ai_mut(b, r).attack.action = ActionVars::FormChange(Vars { timer: 6 });
    set_step(b, r, BeastOutStep::Vanish);
}

/// `sub_80158FA`: movement, reactions and the slower statuses end.
fn drop_statuses(b: &mut Battle, r: ObjectRef) {
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

/// `sub_8014D70`: after 7 ticks the navi vanishes into the beast effect
/// (moved far off the field), which lasts 54 ticks.
fn vanish(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        if !timer_ran_out(b, r) {
            return;
        }
        ai_mut(b, r).status |= status::FORM_CHANGE_SPRITE_HELD;
        b.play_sound(crate::sound::SoundId(0xF7));
        let (pos, alliance) = {
            let o = b.objects.get(r);
            (o.pos, o.alliance)
        };
        if let Some(e) = effect::spawn(b, pos, 0x2E, alliance, 0, 0) {
            let o = b.objects.get_mut(e);
            o.timer = 0x36;
            o.flags |= flags::RUN_WHILE_PAUSED;
            let o = b.objects.get_mut(r);
            o.pos.y = o.pos.y.wrapping_add(0xC0_0000);
            // sub_800AB2E
            b.beast_out_used[alliance as usize] = true;
            // The roar is Gregar's for the Gregar beast and Falzar's
            // otherwise; a 60-tick camera shake follows (its jitter uses
            // the camera's own RNG).
            let gregar = b.turn_transforms[alliance as usize].form == Some(crate::setup::Form::GREGAR_BEAST);
            b.play_sound(crate::sound::SoundId(if gregar { 0x1CC } else { 0x1CD }));
        }
        set_timer(b, r, 0x36);
        ai_mut(b, r).attack.step_init = 4;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    set_step(b, r, BeastOutStep::Emerge);
}

/// `sub_8014E08`: back in the new form (sprite, form, name, overlay, a
/// white flash); 10 ticks later the form's status set-up.
fn emerge(b: &mut Battle, r: ObjectRef, target: Form) {
    // sprite_forceWhitePalette every tick.
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        let current = stats(b, r).form;
        form::take_off_overlay(b, r, current);
        let navi = stats(b, r).navi;
        let sprite = if navi == Navi::MEGAMAN { pdata::form_sprite(target) } else { pdata::navi_sprite(navi) };
        let flip = b.objects.get(r).alliance ^ b.objects.get(r).flip;
        let s = b.objects.sprite_mut(r);
        s.load(sprite);
        // sprite_hasShadow, sprite_setFlip(object_getFlip()), white.
        s.look.shadow = crate::object::sprite::Shadow::Ground;
        s.look.set_flip(flip);
        s.look.white = true;
        let o = b.objects.get_mut(r);
        o.flags &= !flags::NO_SPRITE_UPDATE;
        // object_setAnimation(0), then the sprite restarts it directly.
        o.anim = 0;
        o.anim_loaded = 0xFF;
        b.objects.sprite_mut(r).set_animation(0);
        set_coordinates_from_panel(b, r);
        set_timer(b, r, 10);
        stats_mut(b, r).form = target;
        palette_flash::spawn(b, 14, true, true);
        b.play_sound(crate::sound::SoundId(0x100));
        // sub_8015B22: the form's NameID.
        b.objects.get_mut(r).name_id = 0x1AB + target.0 as u16;
        form::put_on_overlay(b, r, target);
        if let Some(o) = b.objects.get(r).related[1] {
            form_overlay::set_stepping(b, o, form_overlay::Stepping::Normal);
        }
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let old = v.timer;
    v.timer = old.wrapping_sub(1);
    if old >= 2 {
        return;
    }
    let side = b.objects.get(r).alliance;
    let full_synchro = emotion(b, side) == 2;
    reset_status(b, r);
    end_anger(b, r);
    if full_synchro {
        set_mood(b, side, 0xFF);
    }
    clear_invulnerable(b, r);
    set_step(b, r, BeastOutStep::Settle);
}

// ---- Cross -------------------------------------------------------------------

/// The steps of a Cross (`off_8014AB4`), in the attack step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CrossStep {
    /// Stop everything; the sprite holds still from now on.
    Prepare = 0,
    /// The Cross navi's image appears and merges with MegaMan.
    Merge = 4,
    /// MegaMan in the Cross.
    Emerge = 8,
    /// A short pause, then back to idle.
    Settle = 0xC,
}

impl CrossStep {
    fn of(b: &Battle, r: ObjectRef) -> CrossStep {
        match ai(b, r).attack.step {
            0 => CrossStep::Prepare,
            4 => CrossStep::Merge,
            8 => CrossStep::Emerge,
            0xC => CrossStep::Settle,
            s => panic!("Cross step {s:#x}"),
        }
    }
}

fn set_cross_step(b: &mut Battle, r: ObjectRef, step: CrossStep) {
    let a = &mut ai_mut(b, r).attack;
    a.step = step as u8;
    a.step_init = 0;
}

fn cross(b: &mut Battle, r: ObjectRef, target: Form) {
    match CrossStep::of(b, r) {
        CrossStep::Prepare => cross_prepare(b, r),
        CrossStep::Merge => cross_merge(b, r, target),
        CrossStep::Emerge => cross_emerge(b, r, target),
        CrossStep::Settle => cross_settle(b, r),
    }
}

/// `sub_8014B18`: like Beast Out's preparation, but the navi keeps its
/// pose, and its sprite holds still until the change is done.
fn cross_prepare(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).status |= status::FORM_CHANGE_SPRITE_HELD;
    snap_to_future_panel(b, r);
    clear_invulnerable(b, r);
    // sub_800F46C: the standard column patterns face the default way.
    if matches!(b.setup.settings.panel_pattern, 0x38 | 0x30 | 0x3C) {
        b.objects.get_mut(r).flip = 0;
    }
    // (sub_800F2C6: the sprite's facing and the HUD.)
    reset_charge(b, r);
    if ai(b, r).full_synchro_aura.is_some() {
        panic!("ending the Full Synchro aura (sub_80C4C3A) is not implemented yet");
    }
    drop_statuses(b, r);
    ai_mut(b, r).attack.action = ActionVars::FormChange(Vars { timer: 6 });
    if stats(b, r).form == Form(9) && b.objects.get(r).anim == 0x16 {
        panic!("changing Cross from form 9 in animation 0x16 (sub_8014B18) is not implemented yet");
    }
    set_cross_step(b, r, CrossStep::Merge);
}

/// `sub_8014B98`: after 7 ticks the Cross navi's image appears above
/// MegaMan (actor object #0x1B) and merges with him; 49 ticks later he
/// changes. The attack marker counts the ticks MegaMan flashes white.
fn cross_merge(b: &mut Battle, r: ObjectRef, target: Form) {
    if ai(b, r).attack.step_init == 0 {
        if !timer_ran_out(b, r) {
            return;
        }
        cross_merge::spawn(b, r, Navi(target.0), 0x14);
        set_timer(b, r, 0x30);
        let a = &mut ai_mut(b, r).attack;
        a.marker = 0;
        a.step_init = 4;
    }
    let a = &mut ai_mut(b, r).attack;
    if a.marker < 6 {
        a.marker += 1;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    set_timer(b, r, 6);
    set_cross_step(b, r, CrossStep::Emerge);
    ai_mut(b, r).attack.marker = 0;
}

/// `sub_8014BEE`: MegaMan in the Cross (sprite, form, name, overlay); 10
/// ticks later the form's status set-up.
fn cross_emerge(b: &mut Battle, r: ObjectRef, target: Form) {
    if ai(b, r).attack.step_init == 0 {
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        let current = stats(b, r).form;
        form::take_off_overlay(b, r, current);
        let navi = stats(b, r).navi;
        let sprite = if navi == Navi::MEGAMAN { pdata::form_sprite(target) } else { pdata::navi_sprite(navi) };
        b.objects.sprite_mut(r).load(sprite);
        let o = b.objects.get_mut(r);
        o.flags &= !flags::NO_SPRITE_UPDATE;
        // object_setAnimation(0), then the sprite restarts it directly.
        o.anim = 0;
        o.anim_loaded = 0xFF;
        b.objects.sprite_mut(r).set_animation(0);
        set_coordinates_from_panel(b, r);
        set_timer(b, r, 10);
        stats_mut(b, r).form = target;
        // sub_8015B22: the form's NameID.
        b.objects.get_mut(r).name_id = 0x1AB + target.0 as u16;
        form::put_on_overlay(b, r, target);
        b.play_sound(crate::sound::SoundId(0x8D));
        b.play_sound(crate::sound::SoundId(0x77));
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    v.timer = v.timer.wrapping_sub(1);
    if v.timer as i16 > 0 {
        return;
    }
    reset_status(b, r);
    end_anger(b, r);
    let side = b.objects.get(r).alliance;
    set_mood(b, side, 0x80);
    clear_invulnerable(b, r);
    set_cross_step(b, r, CrossStep::Settle);
}

/// `sub_8014CC0`: 21 ticks, then the change is done and the navi idles.
fn cross_settle(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        set_timer(b, r, 0x14);
        ai_mut(b, r).attack.step_init = 4;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    // sub_800AB2E
    let side = b.objects.get(r).alliance as usize;
    b.crossed[side] = true;
    finish(b, r);
}

/// The end of a form change: the flags come off (and battle flag 0x20,
/// which nothing sets), the reactive-defense requests are dropped, and the
/// navi idles.
fn finish(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).status &= !(status::FORM_CHANGE | status::FORM_CHANGE_SPRITE_HELD);
    ai_mut(b, r).requests &= !(request::WEAKNESS_HIT | request::BODY_GUARD_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
    exit_attack_state(b, r);
}

// ---- Beast Out, continued ------------------------------------------------------

/// `sub_8014F04`: 21 ticks, then the change is done and the navi idles.
fn settle(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        set_timer(b, r, 0x14);
        ai_mut(b, r).attack.step_init = 4;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    finish(b, r);
}
