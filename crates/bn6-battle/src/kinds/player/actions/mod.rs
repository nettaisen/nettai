//! Player actions 0x10 and up (the game's `JumpTable80EAC60`) the engine
//! implements itself: movement, and the chip attacks and weapons not yet
//! content. An action the content pack's scripts implement runs as content
//! instead (`behavior`).
//!
//! Every action runs a phase machine on the attack's `step` / `step_init`
//! (which `set_attack` resets); what an action keeps besides that lives
//! in its own state struct in `AttackVars::action`.

pub mod beast_rush;
pub mod cross_change;
pub mod cross_special;
pub mod instant;
pub mod movement;
pub mod navi_chip;
pub mod reactive;
pub mod dimming_chip;
#[cfg(test)]
mod tests;

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
    CrossSpecial(cross_special::Vars),
    CrossChange(cross_change::Vars),
    /// A content action's declared state (see `content`).
    Content(bn6_content_api::ContentState),
}

/// Run attack `action` for the player `r` this tick.
pub fn dispatch(b: &mut Battle, r: ObjectRef, action: super::NaviAction) {
    use super::{EngineAction as E, NaviAction as A};
    match action {
        A::Content(h) => crate::behavior::run_action(b, h, r),
        A::Engine(E::Move) => movement::update(b, r),
        A::Engine(E::DimmingChip) => dimming_chip::update(b, r),
        A::Engine(E::NaviChip) => navi_chip::update(b, r),
        // (Unpaused, the form change's CurAction is the instant chips'.)
        A::Engine(E::InstantChip | E::FormChange) => instant::update(b, r),
        A::Engine(E::CrossSpecial) => cross_special::update(b, r),
        state => unreachable!("{state:?} is not an attack"),
    }
}

/// `object_setDefaultCounterTime`: an attack opens a 16-tick counter
/// window (for players, only in link battles).
pub(crate) fn open_counter_window(b: &mut Battle, r: ObjectRef) {
    if super::ai(b, r).actor_type == crate::actor::ActorType::Player && !super::is_link(b) {
        return;
    }
    super::coll_mut(b, r).counter_timer = 0x10;
}

/// The reactive-defense abort some chip actions check after each phase
/// (requests 0x200, 0x400, 0x8000): a trap's counter takes over
/// (`sub_801056A`).
pub(crate) fn check_reactive_abort(b: &mut Battle, r: ObjectRef) {
    if super::ai(b, r).requests & reactive::TRIGGERS != 0 {
        reactive::counter(b, r);
    }
}
