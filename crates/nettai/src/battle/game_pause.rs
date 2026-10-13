//! A battle held in the game's own pause under nettai's menu ([`super::Battle::pause_in_game`]).
//!
//! Its round pauses itself (its host keys leave the game its START: `HostKeys::pause` off), so the game's pause
//! is the game's: the fight held in its pause state, the battle stepping on in it as the game's does, its sound
//! and what the game moves while paused going on. Nothing here does what pausing does: it gives the game the
//! START the player would, and reads the game's state.
//!
//! - The menu opens (Esc, the pad's Guide, MENU) where START would pause the game (`Battle::pausable`): the
//!   battle is given START for a frame's ticks, and is in the game's pause from that tick. Where it wouldn't
//!   (the custom screen, a dimming, the round over), the battle is held as any other under a menu, still and
//!   silent ([`Phase::Frozen`]).
//! - The player's own START in the fight pauses the game as the game's START does, and the menu opens over
//!   it ([`GamePause::after`] says so).
//! - The menu closes: the battle is given START again, and the game leaves its pause as it does at its
//!   player's START.
//!
//! While the menu is open the player's buttons reach neither the game nor the dummy: the battle has START from
//! here and nothing else.

use nettai_battle::input::keys;
use nettai_frontend::player::Player;

/// Where the battle is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// It runs, the menu closed.
    #[default]
    Running,
    /// The menu opened: START is given until a tick takes it.
    Entering,
    /// In the game's pause.
    Paused,
    /// The menu closed: START is given until a tick takes it.
    Leaving,
    /// START was taken and the game is still paused: a tick without it comes before the next.
    Releasing,
    /// The menu opened where the game wouldn't pause: the player holds the battle, as any battle under a menu.
    Frozen,
}

/// How a battle held in the game's own pause stands.
#[derive(Debug, Default)]
pub struct GamePause {
    phase: Phase,
}

impl GamePause {
    /// The frame's start: what the menu, open or not, asks of the battle as it stands.
    pub fn before(&mut self, menu: bool, player: &mut Player) {
        self.phase = match (self.phase, menu) {
            (Phase::Running, true) if player.battle().pausable() => Phase::Entering,
            (Phase::Running, true) => {
                player.set_paused(true);
                Phase::Frozen
            }
            (Phase::Paused, false) => Phase::Leaving,
            (Phase::Frozen, false) => {
                player.set_paused(false);
                Phase::Running
            }
            (phase, _) => phase,
        };
    }

    /// The buttons the battle is given this frame, besides the player's (who has none while the menu is open).
    pub fn buttons(&self) -> u16 {
        if matches!(self.phase, Phase::Entering | Phase::Leaving) { keys::START } else { 0 }
    }

    /// After the frame's ticks (`ran` of them) as the battle stands: true if the menu is to open, the game
    /// paused by the player's own START.
    pub fn after(&mut self, ran: u32, player: &mut Player, viewer: u8, menu: bool) -> bool {
        let paused = player.battle().paused_by() == Some(viewer & 1);
        let (phase, open) = match self.phase {
            _ if ran == 0 => (self.phase, false),
            Phase::Entering if paused => (Phase::Paused, false),
            // (START reached a battle that didn't pause: it is held as any other.)
            Phase::Entering => {
                player.set_paused(true);
                (Phase::Frozen, false)
            }
            Phase::Leaving if paused => (Phase::Releasing, false),
            Phase::Leaving | Phase::Releasing if !paused => (Phase::Running, false),
            Phase::Releasing => (Phase::Paused, false),
            Phase::Running if paused && !menu => (Phase::Paused, true),
            phase => (phase, false),
        };
        self.phase = phase;
        open
    }

    /// Whether the battle steps under the menu (in the game's pause, or on its way in or out): its ticks are the
    /// frame's sound's. Not where the player holds it.
    pub fn steps(&self) -> bool {
        self.phase != Phase::Frozen
    }

    /// The battle starts over: it is the game's from its start, and nothing holds it.
    pub fn reset(&mut self, player: &mut Player) {
        if self.phase == Phase::Frozen {
            player.set_paused(false);
        }
        self.phase = Phase::Running;
    }
}
