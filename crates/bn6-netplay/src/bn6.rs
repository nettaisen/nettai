//! The battle engine as a rollback [`Game`].
//!
//! [`Battle`] steps on the engine's per-tick input record
//! ([`TickInput`]). Each player contributes their share of it: their
//! buttons, and the frame's events their console produced ([`Bn6Input`]):
//! the link session closing at the end of a round, and, only when checking
//! against a recording that lacks a player's folder, that player's
//! recorded custom-screen results. Carrying the events in the inputs is
//! what makes both peers step each frame with the same ones.
//!
//! Both peers simulate from the same perspective (`RoundSetup::local_side`
//! is part of the shared setup); each presents it for its own player
//! (`Battle::sound_cues_for`, `Battle::banner_for`, ...), see
//! docs/design/rollback.md.

use crate::peer::Observer;
use crate::Game;
use bn6_battle::cues::{CueAction, CueTracker};
use bn6_battle::{Battle, PlayerTick, TickEvents, TickInput};

/// One player's share of a tick's input.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Bn6Input {
    pub tick: PlayerTick,
    /// Events this player's console contributes to the frame (see
    /// `bn6_battle::TickInput`); an input carries them on the frame they
    /// happen only.
    pub events: TickEvents,
}

/// The engine's input record for a frame: buttons by side, and both
/// players' events together.
pub fn tick_input(inputs: &[Bn6Input; 2]) -> TickInput {
    let mut events = TickEvents::default();
    for i in inputs {
        events.link_closed |= i.events.link_closed;
        for (merged, recorded) in events.recorded.iter_mut().zip(&i.events.recorded) {
            if recorded.is_some() {
                *merged = recorded.clone();
            }
        }
    }
    TickInput { players: [inputs[0].tick, inputs[1].tick], events }
}

impl Game for Battle {
    type Input = Bn6Input;

    fn advance(&mut self, inputs: &[Bn6Input; 2]) {
        self.step(&tick_input(inputs));
    }

    fn digest(&self) -> u64 {
        Battle::digest(self)
    }

    fn is_over(&self) -> bool {
        self.round_end().is_some()
    }

    fn blank_input() -> Bn6Input {
        Bn6Input::default()
    }

    /// Buttons carry on; events happen once.
    fn predict(last: &Bn6Input) -> Bn6Input {
        Bn6Input { tick: last.tick, events: TickEvents::default() }
    }
}

/// A game built on a battle.
pub trait HasBattle {
    fn battle(&self) -> &Battle;
}

impl HasBattle for Battle {
    fn battle(&self) -> &Battle {
        self
    }
}

/// Feeds what a peer simulates to a [`CueTracker`] for one viewer: the
/// sound a frontend on that peer plays.
#[derive(Clone, Debug)]
pub struct CueFeed {
    /// The side whose player listens.
    pub viewer: u8,
    pub tracker: CueTracker,
    /// Every action so far, with the peer's frame when it was decided.
    pub log: Vec<(u32, CueAction)>,
    /// The cues of every confirmed frame, in order.
    pub confirmed: Vec<(u32, bn6_battle::SoundCue)>,
}

impl CueFeed {
    pub fn new(viewer: u8, tolerance: u32) -> CueFeed {
        CueFeed { viewer, tracker: CueTracker::new(tolerance), log: Vec::new(), confirmed: Vec::new() }
    }
}

impl<G: Game + HasBattle> Observer<G> for CueFeed {
    fn rolled_back(&mut self, frame: u32) {
        self.tracker.rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G, _resim: bool) {
        self.tracker.simulated(frame, game.battle().sound_cues_for(self.viewer));
        self.log.extend(self.tracker.drain().map(|a| (frame, a)));
    }
    fn confirmed(&mut self, frame: u32, game: &G) {
        self.tracker.confirmed(frame + 1);
        self.confirmed.extend(game.battle().sound_cues_for(self.viewer).iter().map(|&c| (frame, c)));
    }
}
