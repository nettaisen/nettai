//! Actions 0 (battle entry: appear / fade in) and 1 (hand over to the
//! idle controller). See objects-and-player.md §M4.

use super::{
    NaviAction, ai, battle_mode, clear_invulnerable, coll_mut, navi_record, per_player_gauges, set_action, set_invulnerable,
};
use crate::actor::ActorType;
use crate::battle::Battle;
use crate::content::EffectRole;
use crate::object::{ObjectRef, flags};

/// Action 0, `sub_8016380`.
pub(super) fn entry(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).not_counted != 0 {
        return appear(b, r);
    }
    match b.objects.get(r).phase {
        0 => wait_for_fade(b, r),
        4 => fade_in(b, r),
        _ => wait_for_intro(b, r),
    }
}

/// `sub_80164A0`: a navi that isn't one of the battle's combatants
/// appears mid-battle: hidden and untouchable, it flashes in (effect #0
/// looks 6 and, after 20 ticks, 3, with sound 0x129), shows from the next
/// tick on, white fading to its own colors (the color shader: gray at the
/// second timer's level), and after 30 ticks takes control.
fn appear(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).phase {
        // sub_80164C0
        0 => {
            let o = b.objects.get_mut(r);
            o.set_visible(false);
            o.future_panel = o.panel;
            let p = o.panel;
            b.reserve_panel(r, p.x, p.y);
            coll_mut(b, r).region = None;
            b.objects.get_mut(r).phase_init = 4;
            b.sound(crate::content::SoundRole::Appear);
            let o = b.objects.get_mut(r);
            o.timer = 0x14;
            o.timer2 = 0x1E;
            // sprite_setColorShader(white)
            b.objects.sprite_mut(r).look.color_shader = gray(0x1F);
            set_invulnerable(b, r, 0xFFFF);
            let pos = b.objects.get(r).pos;
            flash(b, pos, APPEAR_LOOK);
            b.objects.get_mut(r).phase = 4;
        }
        // sub_8016520
        4 => {
            let o = b.objects.get_mut(r);
            if o.timer != 0 {
                o.timer -= 1;
                if o.timer != 0 {
                    return show(b, r);
                }
                let pos = o.pos;
                flash(b, crate::object::Vec3 { z: pos.z.wrapping_add(0x10_0000), ..pos }, ARRIVE_LOOK);
                b.sound(crate::content::SoundRole::Arrive);
            }
            let o = b.objects.get_mut(r);
            o.timer2 = o.timer2.wrapping_sub(1);
            if o.timer2 != 0 {
                return show(b, r);
            }
            // sprite_zeroColorShader
            b.objects.sprite_mut(r).look.color_shader = 0;
            let o = b.objects.get_mut(r);
            o.phase = 8;
            o.phase_init = 0;
        }
        // sub_801657E
        8 => {
            show_hp_number(b, r);
            clear_invulnerable(b, r);
            let fp = b.objects.get(r).future_panel;
            b.unreserve_panel(r, fp.x, fp.y);
            coll_mut(b, r).region = b.anchor_region();
            set_action(b, r, NaviAction::TakeControl);
        }
        p => panic!("mid-battle appearance phase {p:#x} reads past its table (off_80164B4)"),
    }
}

/// The effects of a mid-battle appearance.
const APPEAR_LOOK: EffectRole = EffectRole::Recovery;
const ARRIVE_LOOK: EffectRole = EffectRole::Deletion;

/// An effect that runs while paused.
fn flash(b: &mut Battle, pos: crate::object::Vec3, look: EffectRole) {
    let look = b.arena_roles().effect(look);
    if let Some(e) = crate::kinds::effect::spawn(b, pos, look, 0, 0, 0) {
        b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
    }
}

/// `loc_801655A`: the navi shows, in the gray of the second timer's level.
fn show(b: &mut Battle, r: ObjectRef) {
    let level = b.objects.get(r).timer2;
    b.objects.sprite_mut(r).look.color_shader = gray(level);
    b.objects.get_mut(r).set_visible(true);
}

/// A color shader of one level in each of its three 5-bit channels.
fn gray(level: u16) -> u16 {
    level.wrapping_mul(1 << 10 | 1 << 5 | 1)
}

/// `sub_80163B4`: the local navi shows at once; the other one queues for
/// its fade-in and waits for the screen fade and its turn.
fn wait_for_fade(b: &mut Battle, r: ObjectRef) {
    let alliance = b.objects.get(r).alliance;
    if b.objects.get(r).phase_init == 0 {
        if !b.is_remote(alliance) {
            let o = b.objects.get_mut(r);
            o.set_visible(true);
            o.phase = 8;
            o.phase_init = 0;
            return;
        }
        b.fadein_enqueue(r);
        b.objects.get_mut(r).phase_init = 4;
    } else if b.round.intro_bits & 0x01 != 0 && b.fadein_is_head(r) {
        b.sound(crate::content::SoundRole::Appear);
        // The sprite starts transparent.
        b.objects.sprite_mut(r).look.alpha = Some(0);
        let o = b.objects.get_mut(r);
        o.timer = 2;
        o.timer2 = 0x10;
        o.phase = 4;
        return;
    }
    b.objects.get_mut(r).set_visible(false);
}

/// `sub_801641A`: fade in over 16 steps of 2 ticks, then leave the queue.
fn fade_in(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.timer = o.timer.wrapping_sub(1);
    if o.timer != 0 {
        return;
    }
    o.timer = 2;
    o.timer2 = o.timer2.wrapping_sub(1);
    if o.timer2 != 0 {
        // Mosaic and alpha follow timer2.
        o.set_visible(true);
        let t = o.timer2 as u8;
        let look = &mut b.objects.sprite_mut(r).look;
        look.mosaic = Some(t);
        look.alpha = Some(16u8.wrapping_sub(t));
        return;
    }
    let look = &mut b.objects.sprite_mut(r).look;
    look.alpha = None;
    look.mosaic = None;
    b.fadein_dequeue(r);
    let o = b.objects.get_mut(r);
    o.phase = 8;
    o.phase_init = 0;
}

/// `sub_8016460`: wait for every navi to be in (intro bit 0x02), then
/// action 1.
fn wait_for_intro(b: &mut Battle, r: ObjectRef) {
    if b.round.intro_bits & 0x02 == 0 {
        return;
    }
    show_hp_number(b, r);
    set_action(b, r, NaviAction::TakeControl);
}

/// `sub_801DC7C(0, 0)` where the navi's HP shows under it: on the other
/// side's console, or on both in battle mode 6. (The original moves the
/// number for NameIDs 0x49..=0x4E, viruses', which no player navi has.)
fn show_hp_number(b: &mut Battle, r: ObjectRef) {
    // (BN5 shows none for its NameID 0x18D, 0x0801339C: a body that hides
    // its HP.)
    if b.content.identity(b.objects.get(r).identity).body.as_ref().is_some_and(|body| body.hides_hp) {
        return;
    }
    let side = b.objects.get(r).alliance;
    let console = if battle_mode(b) == 6 { None } else { Some(side ^ 1) };
    b.show_hp(r, 0, 0, false, console);
}

/// Action 1, `sub_8017888`: hand over to the idle controller (spawning
/// the Beast Out lock-on marker in the battle flag 0x40 mode).
pub(super) fn take_control(b: &mut Battle, r: ObjectRef) {
    if per_player_gauges(b) && navi_record(b, r).actor_type == ActorType::Player && ai(b, r).target_marker.is_none() {
        crate::kinds::target_marker::spawn(b, r);
    }
    set_action(b, r, NaviAction::Idle);
}
