//! Action 0x37: GunDelSol1/2/3/EX (`sub_80EDAE0`). See chips.md §4.
//!
//! A 7-tick wind-up with the gun out, then one hit per tick for 60, 90
//! or 120 ticks on the column two panels ahead (a 2x3 block for EX) under
//! the sun beam, then an 11-tick recovery. The hits are element 5: they
//! drain HP on the target's next update without flinching it or making it
//! flash (field-collision-damage.md §4.5). The damage is 2 per hit, 4 in
//! the sun, whatever the chip data, Atk+ or forms say.

use super::super::{actor_id, ai, ai_mut, clear_flag1, exit_attack_state, set_flag1, stats};
use super::{ActionVars, check_reactive_abort, open_counter_window};
use crate::battle::Battle;
use crate::collision::f1;
use crate::data::attacks;
use crate::kinds::attachment::{self, AttachSlot};
use crate::kinds::common::{facing, set_animation};
use crate::kinds::hitbox::{self, HitboxSpec};
use crate::kinds::sun_beam;
use crate::object::{ObjectRef, PanelPos, Vec3};

/// The action number.
pub const ACTION: u8 = 0x37;

/// GunDelSol's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// Ticks left in the current phase.
    pub timer: u16,
}

/// The phases, on the attack's `step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Gun out, then the beam (`sub_80EDB14`).
    WindUp,
    /// One hit per tick (`sub_80EDBCC`).
    Firing,
    /// Gun away, back to idle (`sub_80EDC78`).
    Recover,
}

impl Phase {
    fn of(step: u8) -> Phase {
        match step {
            0x0 => Phase::WindUp,
            0x4 => Phase::Firing,
            0x8 => Phase::Recover,
            s => panic!("GunDelSol phase {s:#x} reads past its phase table"),
        }
    }

    fn step(self) -> u8 {
        match self {
            Phase::WindUp => 0x0,
            Phase::Firing => 0x4,
            Phase::Recover => 0x8,
        }
    }
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::GunDelSol(v) => v,
        v => panic!("GunDelSol without its state ({v:?})"),
    }
}

fn set_phase(b: &mut Battle, r: ObjectRef, p: Phase) {
    let a = &mut ai_mut(b, r).attack;
    a.step = p.step();
    a.step_init = 0;
}

/// The chip's level (its subtype): S1, S2, S3, EX.
fn level(b: &Battle, r: ObjectRef) -> u8 {
    ai(b, r).attack.variant
}

/// `sub_80EDAE0`: the phase, then the reactive-defense abort.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match Phase::of(ai(b, r).attack.step) {
        Phase::WindUp => wind_up(b, r),
        Phase::Firing => fire(b, r),
        Phase::Recover => recover(b, r),
    }
    check_reactive_abort(b, r);
}

/// The gun's animation steps forward (out, firing, away). Without a gun
/// (its pool was full) the game writes to address 0x10: nothing.
fn advance_gun(b: &mut Battle, r: ObjectRef) {
    if let Some(gun) = ai(b, r).overlay {
        let o = b.objects.get_mut(gun);
        o.anim = o.anim.wrapping_add(1);
    }
}

/// `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
fn wind_up(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        set_flag1(b, r, f1::USING_ACTION);
        set_animation(b, r, 0x0A);
        open_counter_window(b, r);
        let slot = AttachSlot::Overlay(actor_id(b, r));
        attachment::spawn(b, r, 7 + level(b, r), slot);
        b.play_sound(crate::sound::SoundId(0xF8));
        ai_mut(b, r).attack.action = ActionVars::GunDelSol(Vars { timer: 6 });
        ai_mut(b, r).attack.step_init = 4;
        return;
    }
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t > 0 {
        return;
    }
    vars(b, r).timer = attacks::gun_del_sol_firing_ticks(level(b, r));
    advance_gun(b, r);
    let look = attacks::gun_del_sol_beam(stats(b, r).sun, level(b, r));
    let o = b.objects.get(r);
    let offset = Vec3 { x: (facing(o.alliance, o.flip) * 0x50) << 16, y: 0, z: 0 };
    let beam = sun_beam::spawn(b, r, look, offset, AttachSlot::Related(r));
    b.objects.get_mut(r).related[0] = beam;
    set_phase(b, r, Phase::Firing);
}

/// `sub_80EDBCC`: one hit this tick, until the timer runs out.
fn fire(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t < 0 {
        if let Some(beam) = b.objects.get(r).related[0] {
            sun_beam::end(b, beam);
        }
        return set_phase(b, r, Phase::Recover);
    }
    let damage = if stats(b, r).sun { 4 } else { 2 };
    // EX covers two columns.
    let region = if level(b, r) < 3 { 0x04 } else { 0x11 };
    let o = b.objects.get(r);
    let ahead = (2 * facing(o.alliance, o.flip)) as u8;
    let spec = HitboxSpec {
        panel: PanelPos { x: o.panel.x.wrapping_add(ahead), y: o.panel.y },
        element: 5,
        z: 0,
        region,
        hit_effect: 0xFF,
        target: 0x05,
        self_type: 0x2C,
        damage,
        ..HitboxSpec::default()
    };
    hitbox::spawn(b, r, &spec);
    // (The game then reads the types of the panels hit and discards them.)
}

/// `sub_80EDC78`: put the gun away; 11 ticks later, back to idle. The
/// gun and the beam see their slots cleared and end themselves.
fn recover(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step_init == 0 {
        advance_gun(b, r);
        vars(b, r).timer = 10;
        ai_mut(b, r).attack.step_init = 4;
    }
    let v = vars(b, r);
    let t = v.timer as i32 - 1;
    v.timer = t as u16;
    if t >= 0 {
        return;
    }
    clear_flag1(b, r, f1::USING_ACTION);
    set_animation(b, r, 0);
    b.objects.get_mut(r).related[0] = None;
    ai_mut(b, r).overlay = None;
    exit_attack_state(b, r);
}
