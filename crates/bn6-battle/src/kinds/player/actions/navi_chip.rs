//! Action 0x1B, a navi chip (`sub_80EC350`): the navi spawns the chip's
//! dimming controller (`kinds::navi_chip`), registers it for its side
//! and goes back to idle at once; the controller brings the chip's navi.
//! See docs/engine/chips.md §3.6.7.

use crate::actor::AttackVars;
use crate::battle::Battle;
use crate::kinds::navi_chip::{self, Spec};
use crate::kinds::player::{ai, exit_attack_state};
use crate::object::ObjectRef;
use crate::dimming::DimmingChip;

pub const ACTION: u8 = 0x1B;

/// `sub_80EC350`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let a = ai(b, r).attack.clone();
    let controller = spawn_controller(b, r, &a);
    let side = b.objects.get(r).alliance;
    if b.dimming[side as usize].controller.is_none()
        && let Some(c) = controller
    {
        b.register_dimming(side, a.chip_id, c, r);
    }
    exit_attack_state(b, r);
}

/// `sub_80E192C` with the attack variables `a` (r2 the element, r3 the
/// subtype, r4 the params, r6 the damage word, r7 the chip and its bonus):
/// the navi chip's controller for `user`. Action 0x1B and the counter
/// cut-in (`sub_8017AB4`) both call it.
pub(crate) fn spawn_controller(b: &mut Battle, user: ObjectRef, a: &AttackVars) -> Option<ObjectRef> {
    let spec = Spec {
        element: a.element,
        navi: a.variant,
        params: a.params,
        damage: a.damage as u32 | (a.hit_param as u32) << 16,
        chip: DimmingChip { chip: a.chip_id, bonus: a.extra },
    };
    navi_chip::spawn(b, user, spec)
}
