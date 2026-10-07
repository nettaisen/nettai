//! A chip handed off to its controller (EXE6's action 0x1B, `sub_80EC350`;
//! EXE5's 0x41): the navi spawns the chip's controller (its `navi` hook, as
//! a dimming chip's `dimming`), registers it for its side and goes back to
//! idle at once; the controller does the rest. Unlike a dimming chip's
//! (`dimming_chip`), its telop shows the damage the navi used it with. See
//! docs/engine/chips.md §3.6.7.

use nettai_content_api::{DimmingChipSpec, HookCall};

use crate::actor::AttackVars;
use crate::battle::Battle;
use crate::content::ChipUsage;
use crate::kinds::player::{ai, exit_attack_state};
use crate::object::ObjectRef;

/// `sub_80EC350`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r).attack.clone();
    let controller = spawn_controller(b, r, &a);
    let side = b.objects.get(r).alliance;
    if b.dimming[side as usize].controller.is_none()
        && let Some(c) = controller
    {
        b.register_dimming(side, a.chip, c, r);
    }
    exit_attack_state(b, r);
}

/// The chip's controller for `user`, with the attack variables `a` (r2 the
/// element, r6 the damage word, r7 the chip and its bonus; the original's r3
/// and r4, the chip record's subtype and parameters, are the chip's
/// definition here). The action and the counter cut-in (`sub_8017AB4`) both
/// call it.
pub(crate) fn spawn_controller(b: &mut Battle, user: ObjectRef, a: &AttackVars) -> Option<ObjectRef> {
    let damage = a.damage as u32 | (a.hit_param as u32) << 16;
    let chip = b.chip_or_zeroed(a.chip);
    let hook = match b.content.defs.chip(chip).usage {
        ChipUsage::HandOff(f) => f,
        u => panic!("chip {:?} is handed off to its controller, but it is used as {u:?}", b.content.defs.chip(chip).key),
    };
    let spec = DimmingChipSpec { element: a.element, damage, chip: a.chip, bonus: a.extra };
    let controller = crate::behavior::call_hook(b, hook, HookCall::DimmingChip { user, spec }).object();
    // What its telop shows, unless the hook's controller names its own.
    if let Some(c) = controller
        && b.objects.get(c).telop_chip.is_none()
    {
        // (The controller's damage is its damage word's low half.)
        let telop = crate::hud::TelopChip { chip: a.chip, bonus: a.extra, damage: Some(a.damage) };
        b.objects.get_mut(c).telop_chip = Some(telop);
    }
    controller
}
