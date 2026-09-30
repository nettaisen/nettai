//! Action 0x15, a cut-in chip (`sub_80EBD9C`): the navi spawns the
//! chip's dimming controller and registers it for its side, then waits in
//! this action (gated by the dimming) until the dimming is over. The
//! controller does the rest (`dimming`). See docs/engine/chips.md §3.6.

use crate::battle::Battle;
use crate::kinds::{area_grab, invisible, trap_chip};
use crate::kinds::player::{ai, ai_mut, exit_attack_state};
use crate::object::ObjectRef;
use crate::dimming::CutInChip;

pub const ACTION: u8 = 0x15;

/// `sub_80EBD9C`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init != 0 {
        return exit_attack_state(b, r);
    }
    let a = ai(b, r).attack.clone();
    let chip = CutInChip { chip: a.chip_id, bonus: a.extra };
    // off_802CCB4, by the chip's subtype.
    let controller = match a.variant {
        0 => {
            let damage = a.damage as u32 | (a.hit_param as u32) << 16;
            area_grab::spawn(b, r, a.element, a.params, damage, chip)
        }
        1 => invisible::spawn(b, r, a.element, a.params, chip),
        20 => {
            let damage = a.damage as u32 | (a.hit_param as u32) << 16;
            trap_chip::spawn(b, r, a.element, a.params, damage, chip)
        }
        v => panic!("cut-in chip subtype {v} (off_802CCB4) is not implemented yet"),
    };
    let side = b.objects.get(r).alliance;
    if b.dimming[side as usize].controller.is_none()
        && let Some(c) = controller
    {
        b.register_dimming(side, a.chip_id, c, r);
    }
    ai_mut(b, r).attack.step_init = 1;
}
