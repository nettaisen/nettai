//! A battle held in the game's own pause under nettai's menu ([`super::Battle::pause_in_game`]).
//!
//! Its round pauses itself (its host keys leave the game its START: `HostKeys::pause` off), so the game's pause
//! is the game's: the fight held in its pause state, the battle stepping on in it as the game's does, its sound
//! and what the game moves while paused going on. Nothing here does what pausing does: it gives the game the
//! START the player would, and reads the game's state.
//!
//! - The menu opens (Esc, the pad's Guide, MENU) where START would pause the game (`Battle::pausable`): the
//!   battle is given START for a frame's ticks, and is in the game's pause from that tick.
//! - Where START wouldn't pause it, the battle steps on with no buttons ([`Phase::Waiting`]): on its custom
//!   screen, which waits for the player, nothing but time changes (its music, the screen's own animation), and
//!   it stays there; through a dimming or a round's end, until START would pause the fight, and then into the
//!   game's pause (or onto the next round's custom screen, and it waits there).
//! - The player's own START in the fight pauses the game as the game's START does, and the menu opens over
//!   it ([`GamePause::after`] says so).
//! - The menu closes: from the game's pause the battle is given START again, and the game leaves it as it does
//!   at its player's START; from waiting, it goes on as it is.
//!
//! While the menu is open neither the player's buttons nor the dummy's reach the battle: it is given START
//! from here and nothing else.

use nettai_battle::battle::mode;
use nettai_battle::input::keys;
use nettai_frontend::player::Player;

/// Where the battle is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// It runs, the menu closed.
    #[default]
    Running,
    /// The menu is open where START wouldn't pause the game: it steps on with no buttons.
    Waiting,
    /// The menu opened (or waiting found the fight pausable): START is given until a tick takes it.
    Entering,
    /// In the game's pause.
    Paused,
    /// The menu closed: START is given until a tick takes it.
    Leaving,
    /// START was taken and the game is still paused: a tick without it comes before the next.
    Releasing,
}

/// How a battle held in the game's own pause stands.
#[derive(Debug, Default)]
pub struct GamePause {
    phase: Phase,
    /// Whether the dummy's presses are held (they are while the menu is open).
    held: bool,
}

impl GamePause {
    /// The frame's start: what the menu, open or not, asks of the battle as it stands.
    pub fn before(&mut self, menu: bool, player: &mut Player) {
        if self.held != menu {
            self.held = menu;
            player.hold_opponent(menu);
        }
        let pausable = player.battle().pausable();
        self.phase = match (self.phase, menu) {
            (Phase::Running, true) if pausable => Phase::Entering,
            (Phase::Running, true) => Phase::Waiting,
            // (Waiting: into the game's pause once START would pause it; never on the custom screen, which waits
            // for the player.)
            (Phase::Waiting, true) if pausable && player.battle().round.mode != mode::CUSTOM => Phase::Entering,
            (Phase::Waiting, false) => Phase::Running,
            (Phase::Paused, false) => Phase::Leaving,
            (phase, _) => phase,
        };
    }

    /// The buttons the battle is given this frame, besides the player's (who has none while the menu is open).
    pub fn buttons(&self) -> u16 {
        if matches!(self.phase, Phase::Entering | Phase::Leaving) { keys::START } else { 0 }
    }

    /// After the frame's ticks (`ran` of them) as the battle stands: true if the menu is to open, the game
    /// paused by the player's own START.
    pub fn after(&mut self, ran: u32, player: &Player, viewer: u8, menu: bool) -> bool {
        let paused = player.battle().paused_by() == Some(viewer & 1);
        let (phase, open) = match self.phase {
            _ if ran == 0 => (self.phase, false),
            Phase::Entering if paused => (Phase::Paused, false),
            // (START reached a battle that didn't pause: it waits again, and tries once START would.)
            Phase::Entering => (if menu { Phase::Waiting } else { Phase::Running }, false),
            Phase::Leaving if paused => (Phase::Releasing, false),
            Phase::Leaving | Phase::Releasing if !paused => (Phase::Running, false),
            Phase::Releasing => (Phase::Paused, false),
            Phase::Running if paused && !menu => (Phase::Paused, true),
            phase => (phase, false),
        };
        self.phase = phase;
        open
    }

    /// The battle starts over: it is the game's from its start, and nothing holds it (but the dummy's presses,
    /// while the menu is open).
    pub fn reset(&mut self) {
        self.phase = Phase::Running;
    }
}
