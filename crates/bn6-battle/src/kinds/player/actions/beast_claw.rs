//! Action 0x52: the Falzar beast claw (`sub_80EF534`), what a charged
//! Null-family chip becomes in Beast Out (weapon routine 0x1E,
//! `sub_8011E1C`). It runs inside the Beast Out rush, so it strikes from
//! the panel in front of the target: two slashes a panel ahead, each a
//! one-tick hit. See chips.md §4.6.

use super::super::{ai, ai_mut, clear_flag1, exit_attack_state, idle, set_flag1};
use super::{ActionVars, open_counter_window};
use crate::battle::Battle;
use crate::collision::f1;
use crate::kinds::common::{facing, set_animation};
use crate::kinds::effect;
use crate::kinds::hitbox::{self, HitboxSpec};
use crate::kinds::player::panel_coordinates;
use crate::object::{ObjectRef, PanelPos, Vec3};

/// The action number.
pub const ACTION: u8 = 0x52;

/// The claw's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vars {
    /// Ticks left in the current phase.
    pub timer: u16,
    /// Slashes still to come, this one included.
    pub slashes: u16,
}

/// Per slash (first, second): the slash effect, the hit's region shape,
/// and its hit modifier.
const SLASHES: [(u8, u8, u8); 2] = [(0x3A, 0x02, 1), (0x39, 0x04, 3)];

/// The phases, on the attack's `step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Wind up, then slash (`sub_80EF550`).
    Slash,
    /// Follow through, then slash again or finish (`sub_80EF608`).
    Recover,
}

impl Phase {
    fn of(step: u8) -> Phase {
        match step {
            0x0 => Phase::Slash,
            0x4 => Phase::Recover,
            s => panic!("beast claw phase {s:#x} reads past its phase table"),
        }
    }
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::BeastClaw(v) => v,
        v => panic!("beast claw without its state ({v:?})"),
    }
}

/// `sub_8011E1C` (weapon routine 0x1E): 50 damage plus 10 per buster
/// attack level (up to 5), counter strength 0x9E, two slashes.
pub(in crate::kinds::player) fn setup(b: &mut Battle, r: ObjectRef) -> u8 {
    // sub_8012642
    let damage = 0x32 + 0xA * idle::buster_damage(b, r).min(5);
    let a = &mut ai_mut(b, r).attack;
    a.charged = 0;
    a.lockout = 0;
    a.extra = 0;
    a.hit_param = 0x9E;
    a.damage = damage;
    a.element = 0;
    a.action = ActionVars::BeastClaw(Vars { timer: 0, slashes: 2 });
    ACTION
}

/// `sub_80EF534`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match Phase::of(ai(b, r).attack.step) {
        Phase::Slash => slash(b, r),
        Phase::Recover => recover(b, r),
    }
}

/// `sub_80EF550`: claw raised (the overlay too) for 3 ticks, then the
/// slash effect and hit a panel ahead.
fn slash(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        set_flag1(b, r, f1::USING_ACTION);
        ai_mut(b, r).attack.step_init = 1;
        vars(b, r).timer = 3;
        set_animation(b, r, 0x0C);
        if let Some(overlay) = b.objects.get(r).related[1] {
            // A halfword store: the overlay's loaded animation becomes 0.
            let o = b.objects.get_mut(overlay);
            o.anim = 0x0C;
            o.anim_loaded = 0;
        }
    }
    let v = vars(b, r);
    let old = v.timer;
    v.timer = old.wrapping_sub(1);
    if old >= 2 {
        return;
    }
    vars(b, r).timer = 0xC;
    open_counter_window(b, r);
    let a = &mut ai_mut(b, r).attack;
    a.step = 4;
    a.step_init = 0;
    // The first slash and the second sound different.
    let first = vars(b, r).slashes as u8 == 2;
    b.play_sound(crate::sound::SoundId(if first { 0x1C5 } else { 0x1C6 }));
    let k = 2usize.wrapping_sub(vars(b, r).slashes as u8 as usize);
    let (look, region, hit_mod) = *SLASHES.get(k).expect("beast claw slash count");
    let o = b.objects.get(r);
    let (alliance, flip) = (o.alliance, o.flip);
    let ahead = PanelPos { x: o.panel.x.wrapping_add(facing(alliance, flip) as u8), y: o.panel.y };
    let (x, y) = panel_coordinates(ahead.x, ahead.y);
    effect::spawn(b, Vec3 { x, y, z: 0x10_0000 }, look, alliance, 0, 0);
    let a = &ai(b, r).attack;
    let spec = HitboxSpec {
        panel: ahead,
        element: 0,
        z: 0,
        region,
        hit_effect: 0xFF,
        target: 0x05,
        self_type: 0x04,
        damage: a.damage,
        stamina: a.hit_param,
        hit_mod,
        ..HitboxSpec::default()
    };
    hitbox::spawn(b, r, &spec);
}

/// `sub_80EF608`: 12 ticks, then the next slash, or back to idle after
/// the last.
fn recover(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    let old = v.timer;
    v.timer = old.wrapping_sub(1);
    if old >= 2 {
        return;
    }
    let v = vars(b, r);
    v.slashes = v.slashes.wrapping_sub(1);
    if v.slashes != 0 {
        let a = &mut ai_mut(b, r).attack;
        a.step = 0;
        a.step_init = 0;
        return;
    }
    clear_flag1(b, r, f1::USING_ACTION);
    exit_attack_state(b, r);
}
