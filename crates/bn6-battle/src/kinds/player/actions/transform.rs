//! Action 0x1C as a form change (`sub_8014A38`): while the battle is paused
//! at the start of a turn, the navi changes into the form its side asked
//! for (`Battle::turn_transforms`). The transformation sequencer waits for
//! it. The same action reverts a form (`sub_8015614`). See
//! docs/engine/battle-flow.md §3.4.1 and objects-and-player.md §12.9-§12.10.
//!
//! Five sequences, each of four steps (the attack step 0, 4, 8, 0xC):
//!
//! | Target | From | Table | |
//! |---|---|---|---|
//! | 1..=10 | any | `off_8014AB4` | a Cross: the navi's image merges with MegaMan |
//! | 0xB, 0xC | any | `off_8014AC8` | Beast Out |
//! | 0xD..=0x16 | up to 10 | `off_8014ADC` | a Cross Beast, Beast Out's way |
//! | 0xD..=0x16 | past 10 | `off_8014B04` | a Beast's Cross: the Cross navi's image merges with the beast |
//! | 0x17, 0x18 | any | `off_8014AF0` | Beast Over |

use crate::actor::{request, status};
use crate::battle::{Battle, battle_flags};
use crate::collision::{f1, link, timer};
use crate::content::{EffectRole, SoundRole};
use super::ActionVars;
use crate::kinds::common;
use crate::kinds::player::status::end_anger;
use crate::kinds::player::{
    Emotion, ai, ai_mut, clear_flag1, clear_flag2, clear_invulnerable, clear_statuses, coll_mut, emotion,
    exit_attack_state, form, form_of, navi_of, reset_charge, reset_status, set_coordinates_from_panel, set_mood,
    snap_to_future_panel, stats, stats_mut,
};
use crate::kinds::{cross_merge, effect, full_synchro_aura, palette_flash};
use crate::object::{ObjectRef, Vec3, flags};
use crate::setup::{Form, Navi};

/// The five form-change sequences (`sub_8014A38`'s tables).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sequence {
    /// `off_8014AB4`: forms 1..=10.
    Cross,
    /// `off_8014AC8`: forms 0xB and 0xC.
    BeastOut,
    /// `off_8014ADC`: forms 0xD..=0x16 from base or a Cross.
    CrossBeast,
    /// `off_8014B04`: forms 0xD..=0x16 from a Beast form.
    BeastCross,
    /// `off_8014AF0`: forms 0x17 and 0x18.
    BeastOver,
}

impl Sequence {
    fn of(target: Form, current: Form) -> Sequence {
        match target.0 {
            0x17..=0x18 => Sequence::BeastOver,
            0x0D..=0x16 if current.0 > 0x0A => Sequence::BeastCross,
            0x0D..=0x16 => Sequence::CrossBeast,
            0x0B..=0x0C => Sequence::BeastOut,
            _ => Sequence::Cross,
        }
    }
}

/// The steps of every sequence, in the attack step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// Stop everything.
    Prepare = 0,
    /// The beast effect, or the Cross navi's image merging.
    Vanish = 4,
    /// Back in the new form.
    Emerge = 8,
    /// A short pause, then back to idle.
    Settle = 0xC,
}

impl Step {
    fn of(b: &Battle, r: ObjectRef) -> Step {
        match ai(b, r).attack.step {
            0 => Step::Prepare,
            4 => Step::Vanish,
            8 => Step::Emerge,
            0xC => Step::Settle,
            s => panic!("form change step {s:#x} runs off its table (sub_8014A38)"),
        }
    }
}

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

fn set_step(b: &mut Battle, r: ObjectRef, step: Step) {
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

/// Count the timer down; true while it is still above 0.
fn timer_running(b: &mut Battle, r: ObjectRef) -> bool {
    let v = vars(b, r);
    v.timer = v.timer.wrapping_sub(1);
    v.timer as i16 > 0
}

/// `sub_8014A38`: one tick of the form change.
pub(in crate::kinds::player) fn form_change(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    let target = b.turn_transforms[side].form.map(|f| b.content.form_number(f));
    let Some(target) = target.filter(|f| (1..=0x18).contains(&f.0)) else {
        ai_mut(b, r).status &= !status::FORM_CHANGE;
        return;
    };
    let current = form_of(b, r);
    let seq = Sequence::of(target, current);
    if ai(b, r).attack.step == Step::Prepare as u8 && !matches!(ai(b, r).attack.action, ActionVars::FormChange(_)) {
        ai_mut(b, r).attack.action = ActionVars::FormChange(Vars::default());
    }
    match Step::of(b, r) {
        Step::Prepare => prepare(b, r, seq),
        Step::Vanish => match seq {
            Sequence::Cross | Sequence::BeastCross => merge(b, r, seq, target),
            _ => vanish(b, r, seq, target),
        },
        Step::Emerge => emerge(b, r, seq, target),
        Step::Settle => settle(b, r, seq),
    }
    if ai(b, r).status & status::FORM_CHANGE_SPRITE_HELD == 0 {
        common::step_sprite(b, r);
    }
}

/// FuturePanel → panel, the reservation dropped, coordinates and
/// collision panels from it.
fn land(b: &mut Battle, r: ObjectRef) {
    snap_to_future_panel(b, r);
}

/// `sub_800F46C` + `sub_800F2C6`: face the default way under the standard
/// column patterns, and the sprite with it.
pub(in crate::kinds::player) fn face_default(b: &mut Battle, r: ObjectRef) {
    if matches!(b.panel_pattern(), 0x38 | 0x30 | 0x3C) {
        b.objects.get_mut(r).flip = 0;
    }
    let o = b.objects.get(r);
    let facing = o.alliance ^ o.flip;
    b.objects.sprite_mut(r).look.set_flip(facing);
}

/// `sub_80C4C3A` on AIData+0x5C: the Full Synchro aura goes (with none,
/// the game's stores land in BIOS memory).
pub(in crate::kinds::player) fn end_full_synchro_aura(b: &mut Battle, r: ObjectRef) {
    if let Some(aura) = ai(b, r).full_synchro_aura {
        full_synchro_aura::end(b, aura);
    }
}

/// Step 0: `sub_8014D08` (Beast Out), `sub_8014F40` (Cross Beast),
/// `sub_801516C` (Beast Over): onto the destination panel, facing the
/// default way, charge, aura and statuses dropped, links cut; the pose.
/// `sub_8014B18` (Cross) and `sub_80153EC` (a Beast's Cross): the sprite
/// holds still first, and the pose and links stay.
fn prepare(b: &mut Battle, r: ObjectRef, seq: Sequence) {
    let crossing = matches!(seq, Sequence::Cross | Sequence::BeastCross);
    if crossing {
        ai_mut(b, r).status |= status::FORM_CHANGE_SPRITE_HELD;
    }
    land(b, r);
    clear_invulnerable(b, r);
    face_default(b, r);
    reset_charge(b, r);
    end_full_synchro_aura(b, r);
    if !crossing {
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
    }
    drop_statuses(b, r);
    match seq {
        Sequence::Cross => {
            set_timer(b, r, 6);
            // A GroundCross in its animation 0x16 lets go of it.
            if form_of(b, r) == Form(9) && b.objects.get(r).anim == 0x16 {
                b.objects.get_mut(r).anim = 0;
                b.objects.get_mut(r).related[0] = None;
                ai_mut(b, r).overlay = None;
                ai_mut(b, r).status &= !status::FORM_CHANGE_SPRITE_HELD;
                keep_overlay_stepping(b, r);
            }
        }
        Sequence::BeastCross => {
            keep_overlay_stepping(b, r);
            set_timer(b, r, 6);
        }
        _ => {
            b.objects.get_mut(r).anim = 0x11;
            keep_overlay_stepping(b, r);
            set_timer(b, r, 6);
            // (sprite_decompress: graphics.)
        }
    }
    set_step(b, r, Step::Vanish);
}

/// The overlay's Param3 = 1 and flags 0x14, if there is one.
fn keep_overlay_stepping(b: &mut Battle, r: ObjectRef) {
    if let Some(o) = b.objects.get(r).related[1] {
        form::keep_overlay_stepping(b, o);
    }
}

/// `sub_80158FA`: movement, reactions and the slower statuses end.
pub(in crate::kinds::player) fn drop_statuses(b: &mut Battle, r: ObjectRef) {
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

/// Step 4 of Beast Out (`sub_8014D70`), a Cross Beast (`sub_8014FA8`) and
/// Beast Over (`sub_80151D4`): after 7 ticks the navi vanishes into the
/// beast effect (moved far off the field), which lasts 54 ticks.
fn vanish(b: &mut Battle, r: ObjectRef, seq: Sequence, target: Form) {
    if ai(b, r).attack.step_init == 0 {
        if !timer_ran_out(b, r) {
            return;
        }
        ai_mut(b, r).status |= status::FORM_CHANGE_SPRITE_HELD;
        let (pos, alliance) = {
            let o = b.objects.get(r);
            (o.pos, o.alliance)
        };
        let spawned = if seq == Sequence::BeastOver {
            beast_over_effects(b, r, pos, alliance, target)
        } else {
            b.sound(SoundRole::FormChange);
            let look = b.content.defs.roles.effect(EffectRole::FormChange);
            effect::spawn(b, pos, look, alliance, 0, 0).inspect(|&e| {
                let o = b.objects.get_mut(e);
                o.timer = 0x36;
                o.flags |= flags::RUN_WHILE_PAUSED;
            })
        };
        if spawned.is_some() {
            let o = b.objects.get_mut(r);
            o.pos.y = o.pos.y.wrapping_add(0xC0_0000);
            // sub_800AB2E
            b.beast_out_used[alliance as usize] = true;
            // The roar is Gregar's for the Gregar beasts and Falzar's
            // otherwise; the cameras shake at magnitude 2 (`sub_80302B6`),
            // 60 ticks (75 for Beast Over).
            let gregar = match seq {
                Sequence::BeastOut => target == Form::GREGAR_BEAST,
                Sequence::CrossBeast => target.0 < 0x12,
                _ => target == Form::GREGAR_BEAST_OVER,
            };
            b.sound(if gregar { SoundRole::GregarRoar } else { SoundRole::FalzarRoar });
            b.shake_camera_secondary(2, if seq == Sequence::BeastOver { 0x4B } else { 0x3C });
        }
        set_timer(b, r, 0x36);
        if seq == Sequence::BeastOver {
            crate::kinds::beast_over_burst::spawn(b, r);
        }
        ai_mut(b, r).attack.step_init = 4;
    }
    // Beast Over's rumbles. (The game compares the timer as a word with
    // the halfword after it, which its steps leave at 0.)
    if seq == Sequence::BeastOver && matches!(vars(b, r).timer, 0x35 | 0x25) {
        b.sound(SoundRole::BeastOverRumble);
    }
    if !timer_ran_out(b, r) {
        return;
    }
    set_step(b, r, Step::Emerge);
}

/// Beast Over's two effects (`sub_80151D4`): its beast's (the roles
/// `effects.beast_over_gregar` and `effects.beast_over_falzar`) and the
/// blast (`effects.beast_over_blast`, palette by the beast), 32 pixels up.
/// The second only after the first, and the rest only after both.
fn beast_over_effects(b: &mut Battle, _r: ObjectRef, pos: Vec3, alliance: u8, target: Form) -> Option<ObjectRef> {
    let which = target.0 - 0x17;
    let pos = Vec3 { z: pos.z.wrapping_add(0x20_0000), ..pos };
    let roles = &b.content.defs.roles;
    let beast = roles.effect(match which {
        0 => EffectRole::BeastOverGregar,
        1 => EffectRole::BeastOverFalzar,
        _ => panic!("form {target:?} has no Beast Over effect (sub_80151D4)"),
    });
    let blast = roles.effect(EffectRole::BeastOverBlast);
    let first = effect::spawn(b, pos, beast, alliance, 0, 0)?;
    let o = b.objects.get_mut(first);
    o.timer = 0x36;
    o.flags |= flags::RUN_WHILE_PAUSED;
    b.sound(SoundRole::BeastOverRumble);
    let second = effect::spawn(b, pos, blast, alliance, 2 * which, 0)?;
    let o = b.objects.get_mut(second);
    o.timer = 0x45;
    o.flags |= flags::RUN_WHILE_PAUSED;
    Some(second)
}

/// Step 4 of a Cross (`sub_8014B98`) and a Beast's Cross
/// (`sub_801544C`): after 7 ticks the Cross navi's image appears above
/// MegaMan (actor object #0x1B) and merges with him; 49 ticks later he
/// changes. The attack marker counts the ticks MegaMan (and in a Beast's
/// Cross, his overlay) flashes white.
fn merge(b: &mut Battle, r: ObjectRef, seq: Sequence, target: Form) {
    if ai(b, r).attack.step_init == 0 {
        if !timer_ran_out(b, r) {
            return;
        }
        let navi = if seq == Sequence::BeastCross { Navi(target.0 - 0x0C) } else { Navi(target.0) };
        cross_merge::spawn(b, r, navi, 0x14);
        set_timer(b, r, 0x30);
        let a = &mut ai_mut(b, r).attack;
        a.marker = 0;
        a.step_init = 4;
    }
    let next = ai(b, r).attack.marker.wrapping_add(1);
    if next <= 6 {
        ai_mut(b, r).attack.marker = next;
        b.objects.sprite_mut(r).look.white = true;
        if seq == Sequence::BeastCross
            && let Some(o) = b.objects.get(r).related[1]
        {
            b.objects.sprite_mut(o).look.white = true;
        }
    }
    if seq == Sequence::BeastCross
        && next == 7
        && let Some(o) = b.objects.get(r).related[1]
    {
        // sprite_clearFinalPalette
        b.objects.sprite_mut(o).look.white = false;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    set_timer(b, r, 6);
    set_step(b, r, Step::Emerge);
    ai_mut(b, r).attack.marker = 0;
}

/// Step 8 (`sub_8014E08`, `sub_8015040`, `sub_80152C8`, `sub_8014BEE`,
/// `sub_80154C8`): MegaMan in the new form (sprite, form, name, overlay);
/// 10 ticks later the form's status set-up.
fn emerge(b: &mut Battle, r: ObjectRef, seq: Sequence, target: Form) {
    // sprite_forceWhitePalette every tick.
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        match seq {
            Sequence::Cross => {
                b.objects.get_mut(r).related[0] = None;
                ai_mut(b, r).overlay = None;
            }
            Sequence::BeastCross => {
                if let Some(o) = b.objects.get(r).related[1] {
                    b.objects.get_mut(o).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
                }
            }
            _ => {}
        }
        let current = form_of(b, r);
        form::take_off_overlay(b, r, current);
        if seq == Sequence::BeastCross {
            b.objects.get_mut(r).related[0] = None;
            ai_mut(b, r).overlay = None;
        }
        let navi = navi_of(b, r);
        let target_form = b.content.form_numbered(target);
        let sprite =
            if navi == Navi::MEGAMAN { b.content.form(target_form).sprite } else { b.content.navi(stats(b, r).navi).sprite };
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
        b.objects.sprite_mut(r).set_animation(0, &b.content);
        set_coordinates_from_panel(b, r);
        set_timer(b, r, 10);
        if seq == Sequence::BeastOver {
            clear_statuses(b, r);
        }
        stats_mut(b, r).form = target_form;
        if matches!(seq, Sequence::BeastOut | Sequence::CrossBeast | Sequence::BeastOver) {
            palette_flash::spawn(b, 14, true, true);
            b.sound(SoundRole::BeastOut);
        }
        // sub_8015B22: the form's NameID.
        b.objects.get_mut(r).identity = super::super::form_identity(&b.content, target);
        form::put_on_overlay(b, r, target);
        if let Some(o) = b.objects.get(r).related[1] {
            match seq {
                // Its Param3 = 0.
                Sequence::BeastOut | Sequence::BeastOver => form::normal_overlay_stepping(b, o),
                // Its animation 0 and Param3 0 (and in a Cross Beast,
                // flags 0x14).
                Sequence::CrossBeast | Sequence::BeastCross => {
                    b.objects.get_mut(o).anim = 0;
                    form::normal_overlay_stepping(b, o);
                    if seq == Sequence::CrossBeast {
                        b.objects.get_mut(o).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
                    }
                }
                Sequence::Cross => {}
            }
        }
        if matches!(seq, Sequence::Cross | Sequence::BeastCross) {
            b.sound(SoundRole::CrossChange);
            b.sound(SoundRole::CrossChangeChime);
        }
        ai_mut(b, r).attack.step_init = 4;
    }
    if timer_running(b, r) {
        return;
    }
    let side = b.objects.get(r).alliance;
    let full_synchro = emotion(b, side) == Emotion::FullSynchro;
    reset_status(b, r);
    end_anger(b, r);
    match seq {
        Sequence::BeastOut => {
            if full_synchro {
                set_mood(b, side, 0xFF);
            }
        }
        _ => set_mood(b, side, 0x80),
    }
    if seq != Sequence::BeastOver {
        clear_invulnerable(b, r);
    }
    if matches!(seq, Sequence::BeastOut | Sequence::BeastOver | Sequence::BeastCross)
        && let Some(o) = b.objects.get(r).related[1]
    {
        // sprite_clearFinalPalette
        b.objects.sprite_mut(o).look.white = false;
    }
    set_step(b, r, Step::Settle);
}

/// Step 0xC (`sub_8014F04`, `sub_8015128`, `sub_80153B0`, `sub_8014CC0`,
/// `sub_80155CC`): 21 ticks, then the change is done and the navi idles;
/// the Crosses note the side crossed.
fn settle(b: &mut Battle, r: ObjectRef, seq: Sequence) {
    if ai(b, r).attack.step_init == 0 {
        set_timer(b, r, 0x14);
        ai_mut(b, r).attack.step_init = 4;
    }
    if !timer_ran_out(b, r) {
        return;
    }
    if matches!(seq, Sequence::Cross | Sequence::CrossBeast | Sequence::BeastCross) {
        // sub_800AB2E
        let side = b.objects.get(r).alliance as usize;
        b.crossed[side] = true;
    }
    finish(b, r);
}

/// The end of a form change: the flags come off (and battle flag 0x20,
/// which nothing sets), the reactive-defense requests are dropped, and the
/// navi idles.
fn finish(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).status &= !(status::FORM_CHANGE | status::FORM_CHANGE_SPRITE_HELD);
    ai_mut(b, r).requests &=
        !(request::WEAKNESS_HIT | request::BODY_GUARD_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::ANTI_DAMAGE_TRIGGERED);
    exit_attack_state(b, r);
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
        let look = b.content.defs.roles.effect(EffectRole::Deletion);
        if let Some(e) = effect::spawn(b, Vec3 { z: pos.z.wrapping_add(0x14_0000), ..pos }, look, 0, 0, 0) {
            b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
        }
        clear_flag1(b, r, f1::UNTOUCHABLE | f1::SLIDING | f1::FLINCHING | f1::MOVING);
        clear_flag2(b, r, 0x10);
        b.objects.get_mut(r).slide_state = 0;
        let current = form_of(b, r);
        form::take_off_overlay(b, r, current);
        let sprite = b.content.form_data(Form::NONE).sprite;
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
        stats_mut(b, r).form = b.content.form_numbered(Form::NONE);
        b.objects.get_mut(r).identity = super::super::form_identity(&b.content, Form::NONE);
        reset_status(b, r);
        // sub_80143B4: anger ends (the mood is left alone).
        calm_down(b, r);
        clear_invulnerable(b, r);
        form::put_on_overlay(b, r, Form::NONE);
        b.objects.get_mut(r).related[0] = None;
        let a = ai_mut(b, r);
        a.overlay = None;
        // The lock-on marker frees itself once unlinked.
        a.lockon_marker = None;
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

/// `sub_80158CC`: mood 0x80 (stored directly); outside battle mode 1, a
/// Beast Out is used up, and Beast Over exhausts the navi.
fn spend_form(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    b.stats[side].mood = 0x80;
    if crate::kinds::player::battle_mode(b) == 1 {
        return;
    }
    match form_of(b, r).0 {
        0x0B..=0x16 => ai_mut(b, r).beast_out_spent = true,
        0x17.. => {
            // sub_8014466: exhausted, then the mood 0 that exhaustion
            // itself blocks.
            ai_mut(b, r).beast_over_exhausted = true;
            set_mood(b, side as u8, 0);
        }
        _ => {}
    }
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
/// True while it runs.
pub(in crate::kinds::player) fn break_cross(b: &mut Battle, r: ObjectRef) -> bool {
    if !matches!(ai(b, r).attack.action, ActionVars::FormChange(_)) {
        ai_mut(b, r).attack.action = ActionVars::FormChange(Vars::default());
    }
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        b.set_flags(battle_flags::DIMMED);
        b.sound(SoundRole::Fade);
        land(b, r);
        let o = b.objects.get_mut(r);
        o.anim = 2;
        o.anim_loaded = 0xFF;
        crate::kinds::player::refresh_form_overlay(b, r);
        face_default(b, r);
        let pos = b.objects.get(r).pos;
        let look = b.content.defs.roles.effect(EffectRole::Deletion);
        if let Some(e) = effect::spawn(b, Vec3 { z: pos.z.wrapping_add(0x14_0000), ..pos }, look, 0, 0, 0) {
            b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
        }
        clear_flag1(
            b,
            r,
            f1::BUBBLED | f1::DRAG | f1::FROZEN | f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING,
        );
        clear_flag2(b, r, 0x10);
        b.objects.get_mut(r).slide_state = 0;
        keep_overlay_stepping(b, r);
        let current = form_of(b, r);
        form::take_off_overlay(b, r, current);
        let new = match current.0 {
            0..=0x0A => Form::NONE,
            0x0B..=0x11 => Form::GREGAR_BEAST,
            _ => Form::FALZAR_BEAST,
        };
        // sub_800FC9E(MegaMan, the new form).
        let sprite = b.content.form_data(new).sprite;
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
        stats_mut(b, r).form = b.content.form_numbered(new);
        // sub_8015B22
        b.objects.get_mut(r).identity = super::super::form_identity(&b.content, new);
        let side = b.objects.get(r).alliance;
        set_mood(b, side, 0x80);
        reset_status(b, r);
        calm_down(b, r);
        form::put_on_overlay(b, r, new);
        keep_overlay_stepping(b, r);
        clear_invulnerable(b, r);
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        // object_clearCollisionRegion
        coll_mut(b, r).region = None;
        set_timer(b, r, 0x1E);
        ai_mut(b, r).attack.step_init = 4;
    }
    if timer_running(b, r) {
        return true;
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
