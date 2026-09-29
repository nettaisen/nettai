//! Player actions 0x10 and up (the game's `JumpTable80EAC60`): movement,
//! buster, charged shot, and chip attacks.
//!
//! Every action runs a phase machine on the attack's `step` / `step_init`
//! (which `set_attack` resets); what an action keeps besides that lives
//! in its own state struct in `AttackVars::action`.

pub mod beast_claw;
pub mod blank_shot;
pub mod buster;
pub mod beast_rush;
pub mod gun_del_sol;
pub mod movement;
pub mod navi_chip;
pub mod time_freeze;
#[cfg(test)]
mod tests;

pub mod transform;

use crate::battle::Battle;
use crate::object::ObjectRef;

/// The running action's own state. The game keeps it in the shared
/// attack variables, where it outlives the action; nothing reads it from
/// another action.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ActionVars {
    #[default]
    None,
    Move(movement::Vars),
    GunDelSol(gun_del_sol::Vars),
    BeastClaw(beast_claw::Vars),
    FormChange(transform::Vars),
    BlankShot(blank_shot::Vars),
}

/// Run action `action` (>= 0x10) for the player `r` this tick.
pub fn dispatch(b: &mut Battle, r: ObjectRef, action: u8) {
    match action {
        movement::ACTION => movement::update(b, r),
        gun_del_sol::ACTION => gun_del_sol::update(b, r),
        beast_claw::ACTION => beast_claw::update(b, r),
        time_freeze::ACTION => time_freeze::update(b, r),
        blank_shot::ACTION => blank_shot::update(b, r),
        navi_chip::ACTION => navi_chip::update(b, r),
        _ => panic!("player action {action:#x} is not implemented yet"),
    }
}

/// `object_setDefaultCounterTime`: an attack opens a 16-tick counter
/// window (for players, only in link battles).
fn open_counter_window(b: &mut Battle, r: ObjectRef) {
    if super::ai(b, r).actor_type == crate::actor::ActorType::Player && !super::is_link(b) {
        return;
    }
    super::coll_mut(b, r).counter_timer = 0x10;
}

/// `sub_801056A`, the reactive-defense abort some chip actions check after
/// each phase (requests 0x200, 0x400, 0x8000).
fn check_reactive_abort(b: &Battle, r: ObjectRef) {
    use crate::actor::request;
    let f = super::ai(b, r).requests;
    if f & (request::ANTI_DAMAGE_TRIGGERED | request::ANTI_SWORD_TRIGGERED | request::BODY_GUARD_TRIGGERED) != 0 {
        panic!("reactive defensive chips (sub_801056A) are not implemented yet");
    }
}
