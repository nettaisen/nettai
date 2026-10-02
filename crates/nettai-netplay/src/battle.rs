//! The battle engine as a rollback [`Game`].
//!
//! [`Battle`] steps on the engine's per-tick input record
//! ([`TickInput`]). Each player contributes their share of it: their
//! buttons, and the frame's events their console produced ([`PlayerInput`]):
//! the link session closing at the end of a round, and, only when checking
//! against a recording that lacks a player's folder, that player's
//! recorded custom-screen results. Carrying the events in the inputs is
//! what makes both peers step each frame with the same ones.
//!
//! Both peers simulate from the same perspective (`RoundSetup::local_side`
//! is part of the shared setup); each presents it for its own player
//! (`Battle::sound_cues_for`, `Battle::banner_for`, ...), see
//! docs/design/rollback.md.

use std::collections::VecDeque;

use crate::world::{Game, Observer};
use nettai_battle::cues::{CueAction, CueId, CueTracker};
use nettai_battle::{Battle, PlayerTick, SoundCue, TickEvents, TickInput};

/// One player's share of a tick's input.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlayerInput {
    pub tick: PlayerTick,
    /// Events this player's console contributes to the frame (see
    /// `nettai_battle::TickInput`); an input carries them on the frame they
    /// happen only.
    pub events: TickEvents,
}

/// The engine's input record for a frame, from both players' shares by
/// side: buttons by side, and both players' events together.
pub fn tick_input(inputs: [&PlayerInput; 2]) -> TickInput {
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
    type Input = PlayerInput;

    fn step(&mut self, inputs: [&PlayerInput; 2]) {
        Battle::step(self, &tick_input(inputs));
    }

    fn battle(&self) -> &Battle {
        self
    }

    fn battle_mut(&mut self) -> &mut Battle {
        self
    }

    /// Buttons carry on; events happen once.
    fn predict(last: &PlayerInput) -> PlayerInput {
        PlayerInput { tick: last.tick, events: TickEvents::default() }
    }
}

/// Feeds what a peer simulates to a [`CueTracker`] for one viewer: the
/// sound a frontend on that peer plays. Give it to the peer's world
/// ([`crate::world::BattleWorld::with_observer`], shared with the host)
/// and tell it what settles after each advance.
#[derive(Clone, Debug)]
pub struct CueFeed {
    /// The side whose player listens.
    pub viewer: u8,
    pub tracker: CueTracker,
    /// Every action so far, with the frame being simulated when it was
    /// decided.
    pub log: Vec<(u32, CueAction)>,
    /// The play each `log` entry concerns (`log[i]`'s is `cue_ids[i]`): a
    /// play's own identity, or the one a cancel takes back.
    pub cue_ids: Vec<CueId>,
    /// The cues of every confirmed frame, in order.
    pub confirmed: Vec<(u32, SoundCue)>,
    /// The cues of each simulated frame that isn't confirmed yet, from its
    /// latest simulation (frames without cues left out), in frame order.
    unconfirmed: VecDeque<(u32, Vec<SoundCue>)>,
}

impl CueFeed {
    pub fn new(viewer: u8, tolerance: u32) -> CueFeed {
        CueFeed {
            viewer,
            tracker: CueTracker::new(tolerance),
            log: Vec::new(),
            cue_ids: Vec::new(),
            confirmed: Vec::new(),
            unconfirmed: VecDeque::new(),
        }
    }
}

impl<G: Game> Observer<G> for CueFeed {
    fn rolled_back(&mut self, frame: u32) {
        self.tracker.rolled_back(frame);
        while self.unconfirmed.back().is_some_and(|&(f, _)| f >= frame) {
            self.unconfirmed.pop_back();
        }
    }

    fn simulated(&mut self, frame: u32, game: &G) {
        let cues = game.battle().sound_cues_for(self.viewer);
        self.tracker.simulated(frame, cues);
        for (id, a) in self.tracker.drain_identified() {
            self.log.push((frame, a));
            self.cue_ids.push(id);
        }
        if !cues.is_empty() {
            self.unconfirmed.push_back((frame, cues.to_vec()));
        }
    }

    fn confirmed(&mut self, frames: u32, _settled: &Battle) {
        self.tracker.confirmed(frames);
        while let Some((frame, _)) = self.unconfirmed.front()
            && *frame < frames
        {
            let (frame, cues) = self.unconfirmed.pop_front().unwrap();
            self.confirmed.extend(cues.into_iter().map(|c| (frame, c)));
        }
    }
}
