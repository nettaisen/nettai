//! Action 0x33, the blank shot (`sub_80ED748`): what the buster fires when
//! the NaviCust's buster bug picks a blank. The navi raises the buster
//! and a sound plays, but nothing is fired; then the buster's recovery,
//! counted from the navi's own panel (so it is always the shortest).
//! See docs/engine/objects-and-player.md §B6.

use super::{ActionVars, buster};
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common;
use crate::kinds::player::{ai, ai_mut, exit_attack_state, set_flag1};
use crate::object::ObjectRef;

pub const ACTION: u8 = 0x33;

/// The blank shot's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vars {
    /// AIAttackVars+0x10: ticks into the shot, then ticks of recovery
    /// left.
    pub timer: u16,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::BlankShot(v) => v,
        v => panic!("blank shot without its state ({v:?})"),
    }
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.step {
        0 => raise(b, r),
        _ => recover(b, r),
    }
}

/// `sub_80ED764`: raise the buster; the click on the second tick; after 5
/// ticks, the recovery.
fn raise(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        common::set_animation(b, r, 0x0E);
        buster::raise_arm(b, r);
        set_flag1(b, r, f1::USING_ACTION);
        ai_mut(b, r).attack.action = ActionVars::BlankShot(Vars { timer: 0 });
        ai_mut(b, r).attack.step_init = 4;
    }
    if vars(b, r).timer == 1 {
        b.play_sound(crate::sound::SoundId(0xF8));
    }
    let v = vars(b, r);
    v.timer += 1;
    if v.timer > 4 {
        let a = &mut ai_mut(b, r).attack;
        a.step = 4;
        a.step_init = 0;
    }
}

/// `sub_80ED7A2`: the recovery; a move may cut it short.
fn recover(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        let p = b.objects.get(r).panel;
        let ticks = buster::recovery(b, r, p.x, p.y);
        vars(b, r).timer = ticks;
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left < 0 {
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        exit_attack_state(b, r);
        return;
    }
    buster::move_cancel(b, r);
}
