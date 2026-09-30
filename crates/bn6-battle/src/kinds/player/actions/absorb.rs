//! Action 0x58, pulling in the field's obstacles (`sub_80EFCB4`):
//! DustCross's B+Back special (weapon routine 0x2A). A vortex appears in
//! front of the navi; on the 10th tick every obstacle on the field is
//! pulled in (`obstacle::absorb_all`; they fly to the navi and join its
//! absorbed list while it stays in this action); 11 ticks later the navi
//! idles. See docs/engine/field-objects.md §4.4.

use super::ActionVars;
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::player::{ai, ai_mut, clear_flag1, exit_attack_state, set_flag1, stats};
use crate::kinds::{common, effect, obstacle};
use crate::object::{ObjectRef, Vec3, flags};

pub const ACTION: u8 = 0x58;

/// The action's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10.
    pub timer: u16,
    /// AIAttackVars+0x30: the vortex, kept alive while the action runs.
    pub vortex: Option<ObjectRef>,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::Absorb(v) => v,
        v => panic!("absorbing without its state ({v:?})"),
    }
}

/// `sub_8011F84` (weapon routine 0x2A): a 40-tick B+Back cooldown after.
pub(in crate::kinds::player) fn setup(b: &mut Battle, r: ObjectRef) -> u8 {
    ai_mut(b, r).attack.lockout = 0x28;
    ACTION
}

/// `sub_80EFCB4`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.step {
        0 => pull(b, r),
        _ => recover(b, r),
    }
    // The vortex lives while this runs.
    if let ActionVars::Absorb(Vars { vortex: Some(v), .. }) = ai(b, r).attack.action {
        b.objects.get_mut(v).timer = 2;
    }
}

/// `sub_80EFCD8`: the vortex, then the pull on the 10th tick.
fn pull(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        let cross_beast = stats(b, r).form.0 == 0x16;
        common::set_animation(b, r, if cross_beast { 0x19 } else { 0x17 });
        set_flag1(b, r, f1::USING_ACTION | f1::MOVING);
        let o = b.objects.get(r);
        let front = common::facing(o.alliance, o.flip);
        let (back, down) = if cross_beast { (3, -10) } else { (7, 4) };
        let pos = Vec3 {
            x: o.pos.x.wrapping_sub((front * back) << 16),
            y: o.pos.y.wrapping_add(down << 16),
            z: o.pos.z,
        };
        let vortex = effect::spawn(b, pos, 0x63, o.alliance, 0, 0);
        if let Some(e) = vortex {
            b.objects.get_mut(e).flags &= !(flags::RUN_WHILE_PAUSED | flags::RUN_IN_TIME_STOP);
        }
        ai_mut(b, r).attack.action = ActionVars::Absorb(Vars { timer: 9, vortex });
        b.play_sound(crate::sound::SoundId(0xAD));
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left >= 0 {
        return;
    }
    obstacle::absorb_all(b, r);
    vars(b, r).timer = 0xB;
    let a = &mut ai_mut(b, r).attack;
    a.step = 4;
    a.step_init = 0;
}

/// `sub_80EFD5E`: 11 ticks, then idle.
fn recover(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left > 0 {
        return;
    }
    clear_flag1(b, r, f1::MOVING);
    exit_attack_state(b, r);
}
