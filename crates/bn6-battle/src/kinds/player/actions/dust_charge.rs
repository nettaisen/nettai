//! Action 0x57, DustCross's charged shot (`sub_80EFC1C`, weapon routine
//! 0x28): the navi raises the buster, rolls a ball of junk forward
//! (`kinds::dust_ball`) on the second tick, and idles 35 ticks later.

use super::{ActionVars, buster, open_counter_window};
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common;
use crate::kinds::dust_ball;
use crate::kinds::player::idle::buster_damage;
use crate::kinds::player::{ai, ai_mut, exit_attack_state, set_flag1};
use crate::object::{ObjectRef, PanelPos};

pub const ACTION: u8 = 0x57;

/// The action's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10.
    pub timer: u16,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::DustCharge(v) => v,
        v => panic!("DustCross's charged shot without its state ({v:?})"),
    }
}

/// `sub_8011F64` (weapon routine 0x28): 50 damage, plus 10 per buster
/// Attack point (up to 5), Break (secondary element 0x10).
pub(in crate::kinds::player) fn setup(b: &mut Battle, r: ObjectRef) -> u8 {
    // sub_8012642
    let damage = 0x32 + 10 * buster_damage(b, r).min(5);
    let a = &mut ai_mut(b, r).attack;
    a.damage = damage;
    a.hit_param = 0x94;
    a.element = 0x10;
    a.charged = 0;
    a.extra = 0;
    a.lockout = 0;
    ACTION
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).attack.step {
        0 => shoot(b, r),
        _ => recover(b, r),
    }
}

/// `sub_80EFC38`: raise the buster; the ball on the second tick; after 5
/// ticks, the recovery.
fn shoot(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        common::set_animation(b, r, 0x0E);
        buster::raise_arm(b, r);
        open_counter_window(b, r);
        set_flag1(b, r, f1::USING_ACTION);
        b.play_sound(crate::sound::SoundId(0xFF));
        ai_mut(b, r).attack.action = ActionVars::DustCharge(Vars { timer: 0 });
        ai_mut(b, r).attack.step_init = 4;
    }
    if vars(b, r).timer == 1 {
        let o = b.objects.get(r);
        let front = common::facing(o.alliance, o.flip);
        let panel = PanelPos { x: (o.panel.x as i32 + front) as u8, y: o.panel.y };
        let a = &ai(b, r).attack;
        let (element, damage) = (a.element, a.damage as u32 | (a.hit_param as u32) << 16);
        dust_ball::spawn(b, r, panel, element, damage);
    }
    let v = vars(b, r);
    v.timer += 1;
    if v.timer > 4 {
        let a = &mut ai_mut(b, r).attack;
        a.step = 4;
        a.step_init = 0;
    }
}

/// `sub_80EFC8E`: 31 ticks, then idle.
fn recover(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        vars(b, r).timer = 0x1E;
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let left = v.timer as i32 - 1;
    v.timer = left as u16;
    if left < 0 {
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        exit_attack_state(b, r);
    }
}
