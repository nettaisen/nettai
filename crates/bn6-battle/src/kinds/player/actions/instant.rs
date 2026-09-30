//! Action 0x1C outside a pause, chips with an immediate effect
//! (`sub_80EC39C`): the chip's effect (`off_80EC3F0`, by the attack's
//! subtype) runs once and the navi goes back to idle, or, for subtype
//! 0x14, 8 ticks later. The effects are the content pack's
//! (`Hook::InstantChip`: the scripts of the chips with this action, and of
//! weapons that name a subtype no chip has). (Paused, action 0x1C is the
//! form change: `transform`.) See docs/engine/chips.md §1.6.

use bn6_content_api::{Hook, HookCall, InstantChipSpec};

use super::ActionVars;
use crate::battle::Battle;
use crate::kinds::player::{ai, ai_mut, exit_attack_state};
use crate::object::ObjectRef;

pub const ACTION: u8 = 0x1C;

/// The subtype whose effect the navi waits out.
const WAITS: u8 = 0x14;

/// The wait after subtype 0x14.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10.
    pub timer: u16,
}

/// `sub_80EC39C`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step == 0 {
        ai_mut(b, r).attack.step = 1;
        run_effect(b, r);
        // The game reads the subtype again after the effect.
        if ai(b, r).attack.variant != WAITS {
            return exit_attack_state(b, r);
        }
        ai_mut(b, r).attack.action = ActionVars::Instant(Vars { timer: 8 });
    }
    let ActionVars::Instant(v) = &mut ai_mut(b, r).attack.action else {
        panic!("an instant chip's wait without its state");
    };
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left < 0 {
        exit_attack_state(b, r);
    }
}

/// `off_80EC3F0[subtype]`, with the registers `sub_80EC39C` passes: the
/// user's panel and Z, the attack's element and parameters, and its damage
/// word plus the bonus's low byte.
fn run_effect(b: &mut Battle, r: ObjectRef) {
    let a = &ai(b, r).attack;
    let o = b.objects.get(r);
    let spec = InstantChipSpec {
        panel: o.panel,
        element: a.element,
        z: o.pos.z,
        params: a.params,
        damage: (a.damage as u32 | (a.hit_param as u32) << 16).wrapping_add(a.extra as u32 & 0xFF),
    };
    let subtype = a.variant;
    match b.behaviors.hook(Hook::InstantChip(subtype)) {
        Some(hook) => {
            crate::behavior::call_hook(b, hook, HookCall::InstantChip { user: r, spec });
        }
        // Null entries: the game jumps to address 0.
        None if matches!(subtype, 7 | 0x12) => {
            panic!("instant chip subtype {subtype:#x} has no routine in off_80EC3F0 (the game jumps to address 0)")
        }
        None => panic!("instant chip subtype {subtype:#x} (off_80EC3F0) has no script in the content pack"),
    }
}
