//! Action 0x15, a dimming chip (`sub_80EBD9C`): the navi spawns the
//! chip's dimming controller and registers it for its side, then waits in
//! this action (gated by the dimming) until the dimming is over. The
//! controller does the rest (`dimming`). The controllers are the chips'
//! own (`off_802CCB4`, by chip subtype): the chip's `dimming` hook, or a
//! pack record's subtype's registration. See docs/engine/chips.md §3.6.

use nettai_content_api::{DimmingChipSpec, HookCall};

use crate::actor::AttackVars;
use crate::battle::Battle;
use crate::content::ChipUsage;
use crate::kinds::player::{ai, ai_mut, exit_attack_state};
use crate::object::ObjectRef;

/// `sub_80EBD9C`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init != 0 {
        return exit_attack_state(b, r);
    }
    let a = ai(b, r).attack.clone();
    let controller = spawn_controller(b, r, &a);
    let side = b.objects.get(r).alliance;
    if b.dimming[side as usize].controller.is_none()
        && let Some(c) = controller
    {
        b.register_dimming(side, a.chip, c, r);
    }
    // A chip whose game leaves the action on the frame it runs (BN5's
    // 0x080EC318): no wait for the dimming to end.
    if b.chip_rules(a.chip).chip_use.leave_on_use {
        return exit_attack_state(b, r);
    }
    ai_mut(b, r).attack.step_init = 1;
}

/// `off_802CCB4[subtype]`: spawn the dimming controller of the chip the
/// attack variables `a` hold, for `user` (r0/r1 its panel, r2 the element,
/// r6 the damage word, r7 the chip and its bonus; the original's r4, the
/// chip record's parameters, is the chip's definition here). Action
/// 0x15 and the counter cut-in (`sub_8017AB4`) both call it.
pub(crate) fn spawn_controller(b: &mut Battle, user: ObjectRef, a: &AttackVars) -> Option<ObjectRef> {
    let damage = a.damage as u32 | (a.hit_param as u32) << 16;
    // The chip's controller. (The original's table, off_802CCB4, has null
    // entries, 34, 35, 39 and 40, where the game jumps to address 0:
    // Gregar's and Falzar's chips' hooks say so; no chip names the others.)
    let chip = b.chip_or_zeroed(a.chip);
    let hook = match b.content.defs.chip(chip).usage {
        ChipUsage::Dimming(f) => f,
        u => panic!("chip {:?} is a dimming chip's, but it is used as {u:?}", b.content.defs.chip(chip).key),
    };
    let spec = DimmingChipSpec { element: a.element, damage, chip: a.chip, bonus: a.extra };
    let controller = crate::behavior::call_hook(b, hook, HookCall::DimmingChip { user, spec }).object();
    // What its telop shows (the controller's +0x30 and +0x32).
    if let Some(c) = controller {
        b.objects.get_mut(c).telop_chip = Some(crate::hud::TelopChip { chip: a.chip, bonus: a.extra, damage: None });
    }
    controller
}
