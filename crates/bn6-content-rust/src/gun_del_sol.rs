//! Action 0x37: GunDelSol1/2/3/EX (`sub_80EDAE0`). See chips.md §4.
//!
//! A 7-tick wind-up with the gun out, then one hit per tick for 60, 90
//! or 120 ticks on the column two panels ahead (a 2x3 block for EX) under
//! the sun beam, then an 11-tick recovery. The hits are element 5: they
//! drain HP without flinching. The damage is 2 per hit, 4 in the sun.

use bn6_content_api::api::{ActorFields, ObjectFields, OtherFields};
use bn6_content_api::state::TypedState;
use bn6_content_api::{CoreApi, NaviStat, ObjectRef, PanelPos, StatusFlag, Value, Vec3, content_state};

use crate::data::{GUN_DEL_SOL_BEAMS, GUN_DEL_SOL_FIRING_TICKS};
use crate::{Slot, attachment, hitbox, sun_beam};

pub const ACTION: u8 = 0x37;

content_state! {
    /// GunDelSol's own state.
    pub struct State {
        /// Ticks left in the current phase.
        timer: u16,
    }
}

/// The phases, on the attack's step.
const WIND_UP: u8 = 0x0;
const FIRING: u8 = 0x4;
const RECOVER: u8 = 0x8;

fn load(api: &mut dyn CoreApi, me: ObjectRef) -> State {
    State::load(api.action_state_mut(me).expect("a navi runs GunDelSol"))
}

fn store(api: &mut dyn CoreApi, me: ObjectRef, s: State) {
    s.store(api.action_state_mut(me).expect("a navi runs GunDelSol"));
}

fn set_phase(api: &mut dyn CoreApi, me: ObjectRef, step: u8) {
    api.set_step(me, step);
    api.set_step_init(me, 0);
}

fn in_sun(api: &dyn CoreApi, me: ObjectRef) -> bool {
    api.navi_stat(api.alliance(me), NaviStat::Sun) == Value::Bool(true)
}

pub fn update(api: &mut dyn CoreApi, me: ObjectRef) {
    match api.step(me) {
        WIND_UP => wind_up(api, me),
        FIRING => fire(api, me),
        RECOVER => recover(api, me),
        s => panic!("GunDelSol phase {s:#x} reads past its phase table"),
    }
    api.check_reactive_abort(me);
}

/// The gun's animation steps forward (out, firing, away).
fn advance_gun(api: &mut dyn CoreApi, me: ObjectRef) {
    if let Some(gun) = api.overlay(me) {
        let anim = api.anim(gun).wrapping_add(1);
        api.set_anim(gun, anim);
    }
}

/// `sub_80EDB14`: the gun comes out; 6 ticks later the beam lights up.
fn wind_up(api: &mut dyn CoreApi, me: ObjectRef) {
    let level = api.variant(me);
    if api.step_init(me) == 0 {
        api.set_status(me, StatusFlag::UsingAction, true).expect("a navi has collision");
        api.set_animation(me, 0x0A);
        api.open_counter_window(me);
        attachment::spawn(api, me, 7 + level, Slot::Overlay);
        api.play_sound(0xF8);
        store(api, me, State { timer: 6 });
        api.set_step_init(me, 4);
        return;
    }
    let mut s = load(api, me);
    let t = s.timer as i32 - 1;
    s.timer = t as u16;
    store(api, me, s);
    if t > 0 {
        return;
    }
    store(api, me, State { timer: GUN_DEL_SOL_FIRING_TICKS[level as usize] });
    advance_gun(api, me);
    let look = GUN_DEL_SOL_BEAMS[in_sun(api, me) as usize][level as usize];
    let offset = Vec3 { x: (api.facing(me) * 0x50) << 16, y: 0, z: 0 };
    let beam = sun_beam::spawn(api, me, look, offset, Slot::Related);
    api.set_related1(me, beam);
    set_phase(api, me, FIRING);
}

/// `sub_80EDBCC`: one hit this tick, until the timer runs out.
fn fire(api: &mut dyn CoreApi, me: ObjectRef) {
    let mut s = load(api, me);
    let t = s.timer as i32 - 1;
    s.timer = t as u16;
    store(api, me, s);
    if t < 0 {
        if let Some(beam) = api.related1(me) {
            sun_beam::end(api, beam);
        }
        return set_phase(api, me, RECOVER);
    }
    let damage = if in_sun(api, me) { 4 } else { 2 };
    // EX covers two columns.
    let region = if api.variant(me) < 3 { 0x04 } else { 0x11 };
    let ahead = (2 * api.facing(me)) as u8;
    let spec = hitbox::Spec {
        panel: PanelPos { x: api.panel_x(me).wrapping_add(ahead), y: api.panel_y(me) },
        element: 5,
        region,
        hit_effect: 0xFF,
        target: 0x05,
        self_type: 0x2C,
        damage,
        ..hitbox::Spec::default()
    };
    hitbox::spawn(api, me, &spec);
}

/// `sub_80EDC78`: put the gun away; 11 ticks later, back to idle. The gun
/// and the beam see their slots cleared and end themselves.
fn recover(api: &mut dyn CoreApi, me: ObjectRef) {
    if api.step_init(me) == 0 {
        advance_gun(api, me);
        store(api, me, State { timer: 10 });
        api.set_step_init(me, 4);
    }
    let mut s = load(api, me);
    let t = s.timer as i32 - 1;
    s.timer = t as u16;
    store(api, me, s);
    if t >= 0 {
        return;
    }
    api.set_status(me, StatusFlag::UsingAction, false).expect("a navi has collision");
    api.set_animation(me, 0);
    api.set_related1(me, None);
    api.set_overlay(me, None);
    api.exit_attack(me);
}
