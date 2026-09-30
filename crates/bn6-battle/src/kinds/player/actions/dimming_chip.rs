//! Action 0x15, a dimming chip (`sub_80EBD9C`): the navi spawns the
//! chip's dimming controller and registers it for its side, then waits in
//! this action (gated by the dimming) until the dimming is over. The
//! controller does the rest (`dimming`). The controllers are the chips'
//! own, by chip subtype (`off_802CCB4`): the content pack's scripts
//! (`Hook::DimmingChip`), else the engine's. See docs/engine/chips.md §3.6.

use bn6_content_api::{DimmingChipSpec, Hook, HookCall};

use crate::battle::Battle;
use crate::dimming::DimmingChip;
use crate::kinds::player::{ai, ai_mut, exit_attack_state};
use crate::kinds::{invisible, trap_chip};
use crate::object::ObjectRef;

pub const ACTION: u8 = 0x15;

/// `sub_80EBD9C`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init != 0 {
        return exit_attack_state(b, r);
    }
    let a = ai(b, r).attack.clone();
    let chip = DimmingChip { chip: a.chip_id, bonus: a.extra };
    let damage = a.damage as u32 | (a.hit_param as u32) << 16;
    // off_802CCB4, by the chip's subtype.
    let controller = match b.behaviors.hook(Hook::DimmingChip(a.variant)) {
        Some(hook) => {
            let spec = DimmingChipSpec { element: a.element, params: a.params, damage, chip: a.chip_id, bonus: a.extra };
            crate::behavior::call_hook(b, hook, HookCall::DimmingChip { user: r, spec }).object()
        }
        None => match a.variant {
            1 => invisible::spawn(b, r, a.element, a.params, chip),
            20 => trap_chip::spawn(b, r, a.element, a.params, damage, chip),
            v => panic!("dimming chip subtype {v} (off_802CCB4) is not implemented yet"),
        },
    };
    let side = b.objects.get(r).alliance;
    if b.dimming[side as usize].controller.is_none()
        && let Some(c) = controller
    {
        b.register_dimming(side, a.chip_id, c, r);
    }
    ai_mut(b, r).attack.step_init = 1;
}
