//! A battle held in the game's own pause under nettai's menu ([`super::Battle::pause_in_game`]).
//!
//! Its round pauses itself (its host keys leave the game its START: `HostKeys::pause` off), so the game's pause
//! is the game's: the fight held in its pause state, the battle stepping on in it as the game's does, its sound
//! and what the game moves while paused going on. Nothing here does what pausing does: it gives the game the
//! START the player would, and reads the game's state.
//!
//! - The menu opens (Esc, the pad's Guide, MENU) where START would pause the game (`Battle::pausable`): the
//!   battle is given START for a frame's ticks, and is in the game's pause from that tick.
//! - Where START wouldn't pause it, the battle is held where it stands ([`Phase::Holding`],
//!   `Player::set_held`): no tick steps it, so nothing of it changes (the custom screen's HP drain, its cursor,
//!   a dimming's chip, a round's end), and only its sound goes on, the music and what was sounding. (The user:
//!   "pause at start doesn't seem to stop custom hp drain": it stepped on there with no buttons.) Pressing START
//!   there in the game does nothing; nettai's menu holds the battle all the same, and it goes on from there.
//! - Opened in a round's opening (its fade in, its navis' arrival and its banner), the battle steps on through
//!   it with no buttons ([`Phase::Opening`]), and is held as its custom screen begins, before the screen comes
//!   in: at a round's first tick nothing is drawn yet and no music plays, and the menu is there to set the dummy
//!   and the stage up with them in view. Nothing in the opening is the fight's: no gauge fills and no HP
//!   changes. (Practice's own sessions open the menu as the battle starts, and an arena changed under the
//!   menu starts the set over: each steps through its opening so.)
//! - The player's own START in the fight pauses the game as the game's START does, and the menu opens over
//!   it ([`GamePause::after`] says so).
//! - The menu closes: from the game's pause the battle is given START again, and the game leaves it as it does
//!   at its player's START; held, it goes on from the tick it was held at.
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
    /// The menu is open in a round's opening: it steps on with no buttons until its custom screen begins (then
    /// held) or START would pause it.
    Opening,
    /// The menu is open where START wouldn't pause the game: held where it stands, its sound going on.
    Holding,
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
        let battle = player.battle();
        let (pausable, opening) = (battle.pausable(), opening(battle));
        self.phase = match (self.phase, menu) {
            (Phase::Running | Phase::Opening, true) if pausable => Phase::Entering,
            (Phase::Running | Phase::Opening, true) if opening => Phase::Opening,
            (Phase::Running | Phase::Opening, true) => Phase::Holding,
            (Phase::Opening | Phase::Holding, false) => Phase::Running,
            (Phase::Paused, false) => Phase::Leaving,
            (phase, _) => phase,
        };
        player.set_held(self.phase == Phase::Holding);
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
            // (START reached a battle that didn't pause: the next frame finds where it is.)
            Phase::Entering => (Phase::Running, false),
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
    /// while the menu is open: under it the new set steps through its opening and is held as its custom screen begins).
    pub fn reset(&mut self, player: &mut Player) {
        self.phase = Phase::Running;
        player.set_held(false);
    }
}

/// Whether `battle` is in a round's opening: its fade in, its navis' arrival and its banner, before its custom
/// screen (which is held as it begins to come in: its HP bug drains from the screen's first tick).
fn opening(battle: &nettai_battle::battle::Battle) -> bool {
    matches!(battle.round.mode, mode::INTRO | mode::BANNER) && battle.round.top == nettai_battle::battle::top::RUNNING
}
