//! Action 0x1C outside a pause, chips with an immediate effect
//! (`sub_80EC39C`): the effect (`off_80EC3F0`, by the attack's subtype)
//! runs once and the navi goes back to idle, or, for a weapon's own effect
//! (the original's subtype 0x14, which no chip has), 8 ticks later. The
//! effects are content's: the attack's `instant`, which chip use sets from
//! the chip (its `instant` hook, or a pack record's subtype's registration)
//! and a weapon with an effect of its own (TenguCross's wind) from its
//! definition. (Paused, action 0x1C is the form change: `transform`.) See
//! docs/engine/chips.md §1.6.

use bn6_content_api::{FnId, HookCall, InstantChipSpec};

use super::ActionVars;
use crate::battle::Battle;
use crate::kinds::player::{ai, ai_mut, exit_attack_state};
use crate::object::ObjectRef;

pub const ACTION: u8 = 0x1C;

/// What the instant chips' action runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// A chip's effect: it runs, and the navi idles.
    Runs(FnId),
    /// A weapon's own effect (the original's subtype 0x14, TenguCross's
    /// wind): it runs, and the navi waits 8 ticks.
    RunsThenWaits(FnId),
    /// A pack record's subtype nothing implements yet.
    Unported(u8),
}

/// The wait after a weapon's effect.
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
        // (The game reads the subtype again after the effect: only the
        // weapon's, 0x14, waits.)
        if !matches!(ai(b, r).attack.instant, Some(Effect::RunsThenWaits(_))) {
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
    match a.instant {
        Some(Effect::Runs(hook) | Effect::RunsThenWaits(hook)) => {
            crate::behavior::call_hook(b, hook, HookCall::InstantChip { user: r, spec });
        }
        // Null entries: the game jumps to address 0.
        Some(Effect::Unported(subtype)) if matches!(subtype, 7 | 0x12) => {
            panic!("instant chip subtype {subtype:#x} has no routine in off_80EC3F0 (the game jumps to address 0)")
        }
        Some(Effect::Unported(subtype)) => {
            panic!("instant chip subtype {subtype:#x} (off_80EC3F0) has no script in the content pack")
        }
        None => panic!("the instant chips' action without an effect (neither a chip's nor a weapon's)"),
    }
}
