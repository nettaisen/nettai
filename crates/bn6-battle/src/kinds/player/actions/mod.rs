//! Player actions 0x10 and up (the game's `JumpTable80EAC60`): movement,
//! buster, charged shot, and chip attacks.

use crate::battle::Battle;
use crate::object::ObjectRef;

/// Run action `action` (>= 0x10) for the player `r` this tick.
pub fn dispatch(_b: &mut Battle, _r: ObjectRef, action: u8) {
    panic!("player action {action:#x} is not implemented yet");
}
