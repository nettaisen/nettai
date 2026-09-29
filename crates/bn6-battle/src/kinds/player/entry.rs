//! Actions 0 (battle entry: appear / fade in) and 1 (hand over to the
//! idle controller). See objects-and-player.md §M4.

use super::{ai, is_mode_40, navi_record};
use crate::actor::ActorType;
use crate::battle::Battle;
use crate::object::{ObjectRef, flags};

/// Action 0, `sub_8016380`.
pub(super) fn entry(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).not_counted != 0 {
        panic!("mid-battle appearance (sub_80164A0) is not implemented yet");
    }
    match b.objects.get(r).phase {
        0 => wait_for_fade(b, r),
        4 => fade_in(b, r),
        _ => wait_for_intro(b, r),
    }
}

/// `sub_80163B4`: the local navi shows at once; the other one queues for
/// its fade-in and waits for the screen fade and its turn.
fn wait_for_fade(b: &mut Battle, r: ObjectRef) {
    let alliance = b.objects.get(r).alliance;
    if b.objects.get(r).phase_init == 0 {
        if !b.is_remote(alliance) {
            let o = b.objects.get_mut(r);
            o.flags |= flags::VISIBLE;
            o.phase = 8;
            o.phase_init = 0;
            return;
        }
        b.fadein_enqueue(r);
        b.objects.get_mut(r).phase_init = 4;
    } else if b.round.intro_bits & 0x01 != 0 && b.fadein_is_head(r) {
        // Sound 0x94; the sprite starts transparent.
        let o = b.objects.get_mut(r);
        o.timer = 2;
        o.timer2 = 0x10;
        o.phase = 4;
        return;
    }
    b.objects.get_mut(r).flags &= !flags::VISIBLE;
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
        o.flags |= flags::VISIBLE;
        return;
    }
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
    // Battle mode 6 or the remote navi: the HP HUD (sub_801DC7C).
    let o = b.objects.get_mut(r);
    o.action = 1;
    o.phase = 0;
    o.phase_init = 0;
}

/// Action 1, `sub_8017888`: hand over to the idle controller (spawning
/// the Beast Out lock-on marker in the battle flag 0x40 mode).
pub(super) fn take_control(b: &mut Battle, r: ObjectRef) {
    if is_mode_40(b) && navi_record(b, r).actor_type == ActorType::Player && ai(b, r).lockon_marker.is_none() {
        panic!("Beast Out lock-on marker (sub_80E1620) is not implemented yet");
    }
    let o = b.objects.get_mut(r);
    o.action = 8;
    o.phase = 0;
    o.phase_init = 0;
}
