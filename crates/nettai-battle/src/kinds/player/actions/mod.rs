//! Player actions 0x10 and up (the game's `JumpTable80EAC60`) the engine
//! implements itself: movement, and the chip attacks and weapons not yet
//! content. An action the content pack's scripts implement runs as content
//! instead (`behavior`).
//!
//! Every action runs a phase machine on the attack's `step` / `step_init`
//! (which `set_attack` resets); what an action keeps besides that lives
//! in its own state struct in `AttackVars::action`.

pub mod navi_switch;
pub mod instant;
pub mod lockon;
pub mod movement;
pub mod navi_chip;
pub mod reactive;
pub mod dimming_chip;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod beast_chips_tests;

pub mod transform;

use crate::battle::Battle;
use crate::object::ObjectRef;

/// The running action's own state. The game keeps it in the shared
/// attack variables, where it outlives the action; nothing reads it from
/// another action.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActionVars {
    #[default]
    None,
    Move(movement::Vars),
    FormChange(transform::Vars),
    Instant(instant::Vars),
    NaviSwitch(navi_switch::Vars),
    /// A content action's declared state, of this layout: its values are
    /// the actor's block in the actors' arena (`Actors::action_state`).
    Content(nettai_content_api::StateId),
}

/// Run attack `action` for the player `r` this tick.
pub fn dispatch(b: &mut Battle, r: ObjectRef, action: super::NaviAction) {
    use super::{EngineAction as E, NaviAction as A};
    match action {
        // (Unpaused, a form change's CurAction is the instant chips': a
        // form's change action runs only from the pause handler.)
        A::Content(h) if b.content.defs.is_change_action(h) => instant::update(b, r),
        A::Content(h) => crate::behavior::run_action(b, h, r),
        A::Engine(E::Move) => movement::update(b, r),
        A::Engine(E::DimmingChip) => dimming_chip::update(b, r),
        A::Engine(E::NaviChip) => navi_chip::update(b, r),
        // (Unpaused, the form change's CurAction is the instant chips'.)
        A::Engine(E::InstantChip | E::FormChange) => instant::update(b, r),
        state => unreachable!("{state:?} is not an attack"),
    }
}

/// `object_setCounterTime`: an attack opens a counter window of `ticks`
/// (for players, only in link battles); `object_setDefaultCounterTime`'s
/// is 16 ticks.
pub(crate) fn open_counter_window(b: &mut Battle, r: ObjectRef, ticks: u8) {
    if super::ai(b, r).actor_type == crate::actor::ActorType::Player && !super::is_link(b) {
        return;
    }
    super::coll_mut(b, r).counter_timer = ticks;
}

/// The reactive-defense abort some chip actions check after each phase
/// (requests 0x200, 0x400, 0x8000): a trap's counter takes over
/// (`sub_801056A`).
pub(crate) fn check_reactive_abort(b: &mut Battle, r: ObjectRef) {
    if super::ai(b, r).requests & reactive::TRIGGERS != 0 {
        reactive::counter(b, r);
    }
}
