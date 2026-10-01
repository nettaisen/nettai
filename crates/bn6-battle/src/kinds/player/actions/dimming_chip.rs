//! Action 0x15, a dimming chip (`sub_80EBD9C`): the navi spawns the
//! chip's dimming controller and registers it for its side, then waits in
//! this action (gated by the dimming) until the dimming is over. The
//! controller does the rest (`dimming`). The controllers are the chips'
//! own (`off_802CCB4`, by chip subtype): the chip's `dimming` hook, or a
//! pack record's subtype's registration. See docs/engine/chips.md §3.6.

use bn6_content_api::{DimmingChipSpec, HookCall};

use crate::actor::AttackVars;
use crate::battle::Battle;
use crate::content::ChipUsage;
use crate::kinds::player::{ai, ai_mut, exit_attack_state};
use crate::object::ObjectRef;

pub const ACTION: u8 = 0x15;

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
    ai_mut(b, r).attack.step_init = 1;
}

/// `off_802CCB4[subtype]`: spawn the dimming controller of the chip the
/// attack variables `a` hold, for `user` (r0/r1 its panel, r2 the element,
/// r4 the params, r6 the damage word, r7 the chip and its bonus). Action
/// 0x15 and the counter cut-in (`sub_8017AB4`) both call it.
pub(crate) fn spawn_controller(b: &mut Battle, user: ObjectRef, a: &AttackVars) -> Option<ObjectRef> {
    let damage = a.damage as u32 | (a.hit_param as u32) << 16;
    // The chip's controller. (off_802CCB4's subtypes 34, 35, 39 and 40
    // are null: the game jumps to address 0.)
    let chip = a.chip.or_else(|| b.content.chip_numbered(0)).expect("the pack's chip 0 (a zeroed chip field reads it)");
    let hook = match b.content.defs.chip(chip).usage {
        ChipUsage::Dimming(f) => f,
        ChipUsage::Unported(crate::content::Unported::Dimming(subtype)) => {
            panic!("dimming chip subtype {subtype} (off_802CCB4) is not implemented yet")
        }
        u => panic!("chip {:?} is a dimming chip's, but it is used as {u:?}", b.content.defs.chip(chip).key),
    };
    // (The numeric API's chip: 0 for none.)
    let chip = b.api_chip_field(a.chip, 0);
    let spec = DimmingChipSpec { element: a.element, params: a.params, damage, chip, bonus: a.extra };
    let controller = crate::behavior::call_hook(b, hook, HookCall::DimmingChip { user, spec }).object();
    // What its telop shows (the controller's +0x30 and +0x32).
    if let Some(c) = controller {
        b.objects.get_mut(c).telop_chip = Some(crate::hud::TelopChip { chip: a.chip, bonus: a.extra, damage: None });
    }
    controller
}
