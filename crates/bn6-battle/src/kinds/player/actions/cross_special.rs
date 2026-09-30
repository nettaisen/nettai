//! Action 0x59, the end of the Cross special (`sub_80EFDB2`): the dark
//! chips' auto-battle is over (its ticks ran out). The navi flashes white,
//! lands where it was going with its collision off, and after 30 ticks
//! gets its collision back, loses the invulnerability and pending attacks,
//! flashes for 120 ticks, and its HP bug worsens by 4 (at most 7).

use super::ActionVars;
use crate::battle::Battle;
use crate::collision::{f1, timer};
use crate::kinds::player::{
    ai, ai_mut, clear_flag1, clear_flag2, coll_mut, exit_attack_state, set_coordinates_from_panel, set_flag1,
    stats_mut,
};
use crate::object::{ObjectRef, Vec3, flags};

pub const ACTION: u8 = 0x59;

/// The action's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10: ticks left.
    pub timer: u16,
}

const SOUND: crate::sound::SoundId = crate::sound::SoundId(0x8E);
/// The effect #0 look over the navi, 20 pixels up.
const LOOK: u8 = 3;
const LOOK_Z: i32 = 0x14 << 16;
const TICKS: u16 = 0x1E;

/// `sub_80EFDB2`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    // sprite_forceWhitePalette (every tick).
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        b.play_sound(SOUND);
        coll_mut(b, r).region = 0;
        let o = b.objects.get_mut(r);
        o.panel = o.future_panel;
        let p = o.panel;
        b.reserve_panel(r, p.x, p.y);
        set_coordinates_from_panel(b, r);
        b.update_collision_panels(r);
        super::transform::face_default(b, r);
        let pos = b.objects.get(r).pos;
        let at = Vec3 { z: pos.z.wrapping_add(LOOK_Z), ..pos };
        if let Some(e) = crate::kinds::effect::spawn(b, at, LOOK, 0, 0, 0) {
            b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
        }
        clear_flag1(
            b,
            r,
            f1::BUBBLED | f1::FROZEN | f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING,
        );
        clear_flag2(b, r, 0x10);
        ai_mut(b, r).status &= !crate::actor::status::HEAT_TRAP;
        let a = &mut ai_mut(b, r).attack;
        a.action = ActionVars::CrossSpecial(Vars { timer: TICKS });
        a.step_init = 4;
    }
    let ActionVars::CrossSpecial(v) = &mut ai_mut(b, r).attack.action else {
        panic!("the Cross special's end without its state");
    };
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left > 0 {
        return;
    }
    coll_mut(b, r).region = 1;
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    use crate::actor::request;
    ai_mut(b, r).requests &=
        !(request::MODE9_A | request::ALT_CHIP | request::BACK_SPECIAL | request::CHARGED_CHIP | request::CHIP | request::BUSTER | request::FORCED_CHARGED_SHOT);
    // sub_800EB08
    coll_mut(b, r).status_timers[timer::INVULNERABLE] = 0;
    clear_flag1(b, r, f1::INVULNERABLE);
    // sub_801A66C: 120 ticks of flashing.
    coll_mut(b, r).status_timers[timer::FLASH] = 0x78;
    set_flag1(b, r, f1::FLASHING);
    let s = stats_mut(b, r);
    s.bugs.hp_drain = (s.bugs.hp_drain + 4).min(7);
    exit_attack_state(b, r);
}
