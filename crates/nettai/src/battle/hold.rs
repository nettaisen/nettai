//! A battle held under nettai's menu ([`super::Battle`]'s, wherever the menu opens).
//!
//! One hold, everywhere (the user: "pause during custom screen vs not custom screen is so janky now"; they
//! chose to always hold the battle itself and keep the scenery moving): mid-fight, on the custom screen, in a
//! dimming, at a round's end and in a round's opening, the battle is held where it stands (`Player::set_held`).
//! No tick steps it, no button reaches it and no cue sounds, so nothing of it changes (the custom screen's HP
//! drain, its cursor, a dimming's chip, a round's end); its music and what was sounding go on, and its stage's
//! scenery (the background's animation and scroll, the panels' own animations) moves on a clock of the
//! picture's own, as the game's own pause shows them mid-fight. The menu closes: the battle goes on from the
//! tick it was held at, the scenery on from where the hold left it. The game's own pause never runs: START in
//! the fight is the host's (`HostKeys::pause`), and what it opens is nettai's menu.
//!
//! One exception, a battle (re)started with the menu open (Practice's own sessions open the menu as the battle
//! starts; an arena or a Dummy's build changed under the menu starts the set over): at a round's first tick
//! nothing is drawn yet and no music plays, so the battle steps through its round's opening (its fade in, its
//! navis' arrival and its banner) with no buttons, neither the player's nor the dummy's ([`Phase::Opening`]),
//! and is held as its custom screen begins, before the screen comes in, with the dummy and the stage in view.
//! Nothing in the opening is the fight's: no gauge fills and no HP changes. Opened in an opening already under
//! way, the menu holds the battle there as anywhere.
//!
//! A battle in real time with another player (an online match) is never held: its menu says "the match keeps
//! running".

use nettai_battle::battle::mode;
use nettai_frontend::player::Player;

/// Where the battle is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// It runs, the menu closed.
    #[default]
    Running,
    /// The menu has been open since the battle's start: it steps through its round's opening with no buttons
    /// until its custom screen begins, and is held from there.
    Opening,
    /// Held where it stands under the menu.
    Holding,
}

/// How a battle under the menu stands.
#[derive(Debug, Default)]
pub struct Hold {
    phase: Phase,
    /// Whether the dummy's presses are held (they are while the menu is open: through an opening it steps on).
    dummy_held: bool,
}

impl Hold {
    /// The frame's start: what the menu, open or not, asks of the battle as it stands.
    pub fn before(&mut self, menu: bool, player: &mut Player) {
        if player.real_time() {
            return;
        }
        if self.dummy_held != menu {
            self.dummy_held = menu;
            player.hold_opponent(menu);
        }
        let opening = opening(player.battle());
        self.phase = match (self.phase, menu) {
            (_, false) => Phase::Running,
            (Phase::Running, true) if opening && player.ticks() == 0 => Phase::Opening,
            (Phase::Opening, true) if opening => Phase::Opening,
            (_, true) => Phase::Holding,
        };
        player.set_held(self.phase == Phase::Holding);
    }

    /// The battle starts over: nothing holds it (but the dummy's presses, while the menu is open: under it the
    /// new set steps through its opening and is held as its custom screen begins).
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
