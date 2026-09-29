//! Action 0x1C outside a pause, chips with an immediate effect
//! (`sub_80EC39C`): the chip's routine (`off_80EC3F0`, by subtype) runs
//! once and the navi goes back to idle, or, for subtype 0x14, 8 ticks
//! later. (Paused, action 0x1C is the form change: `transform`.)
//! See docs/engine/chips.md §1.6.

use super::ActionVars;
use crate::battle::Battle;
use crate::hud::CustomGauge;
use crate::kinds::player::{ai, ai_mut, exit_attack_state, per_player_gauges};
use crate::object::ObjectRef;

pub const ACTION: u8 = 0x1C;

/// The wait after subtype 0x14.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vars {
    /// AIAttackVars+0x10.
    pub timer: u16,
}

/// `sub_80EC39C`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let a = &ai(b, r).attack;
    if a.step == 0 {
        let variant = a.variant;
        ai_mut(b, r).attack.step = 1;
        match variant {
            5 => full_custom(b, r),
            v => panic!("chip effect {v:#x} (off_80EC3F0) is not implemented yet"),
        }
        if variant != 0x14 {
            return exit_attack_state(b, r);
        }
        ai_mut(b, r).attack.action = ActionVars::Instant(Vars { timer: 8 });
    }
    let ActionVars::Instant(v) = &mut ai_mut(b, r).attack.action else {
        panic!("a chip effect's wait without its state");
    };
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left < 0 {
        exit_attack_state(b, r);
    }
}

/// `sub_800AF34`, FullCust: the custom gauge is full.
fn full_custom(b: &mut Battle, _r: ObjectRef) {
    if per_player_gauges(b) {
        panic!("FullCust with per-player gauges (sub_802E032) is not implemented yet");
    }
    // sub_801DFA2
    b.gauge.value = CustomGauge::FULL;
}
