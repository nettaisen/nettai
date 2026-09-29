//! Action 0x1C as a form change (`sub_8014A38`): while the battle is paused
//! at the start of a turn, the navi changes into the form its side asked
//! for (`Battle::turn_transforms`). The transformation sequencer waits for
//! it. Only Beast Out is implemented. See docs/engine/battle-flow.md
//! §3.4.1.

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
use crate::kinds::{effect, form_overlay, palette_flash};
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
        _ => panic!("Cross changes (sub_8014B18) are not implemented yet"),
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
        // Sound 0xF7.
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
            // Sound 0x1CC (Gregar) or 0x1CD (Falzar), and a 60-tick
            // camera shake (its jitter uses the camera's own RNG).
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
    if ai(b, r).attack.step_init == 0 {
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
        palette_flash::spawn(b, 14, true, true);
        // Sound 0x100.
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

/// `sub_8014F04`: 21 ticks, then the change is done and the navi idles.
fn settle(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        set_timer(b, r, 0x14);
        ai_mut(b, r).attack.step_init = 4;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    // (It also clears battle flag 0x20, which nothing sets.)
    ai_mut(b, r).status &= !(status::FORM_CHANGE | status::FORM_CHANGE_SPRITE_HELD);
    ai_mut(b, r).requests &= !(request::WEAKNESS_HIT | request::BODY_GUARD_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
    exit_attack_state(b, r);
}
