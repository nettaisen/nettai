//! The battle engine as a rollback [`Game`].
//!
//! [`Battle`] steps on the engine's per-tick input record
//! ([`TickInput`]). Each player contributes their share of it: their
//! buttons, and the frame's events their console produced ([`PlayerInput`]):
//! the link session closing at the end of a round. Carrying the events in
//! the inputs is what makes both peers step each frame with the same ones.
//! On the wire (`WireInput`), the events are a tick's flags.
//!
//! Both peers simulate from the same perspective (`RoundSetup::local_side`
//! is part of the shared setup); each presents it for its own player
//! (`Battle::sound_cues_for`, `Battle::banner_for`, ...), see
//! docs/design/rollback.md.

use std::collections::VecDeque;
use std::io;

use crate::protocol::WireInput;
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
    let events = TickEvents { link_closed: inputs.iter().any(|i| i.events.link_closed) };
    TickInput { players: [inputs[0].tick, inputs[1].tick], events }
}

/// A player's input on the wire: the buttons, and the events as flags
/// ([`flags`]).
impl WireInput for PlayerInput {
    fn encode(&self) -> (u16, u8) {
        let TickEvents { link_closed } = &self.events;
        (self.tick.held, if *link_closed { flags::LINK_CLOSED } else { 0 })
    }

    fn decode(held: u16, f: u8) -> io::Result<PlayerInput> {
        if f & !flags::ALL != 0 {
            return Err(crate::protocol::invalid("unknown event flags"));
        }
        Ok(PlayerInput { tick: PlayerTick { held }, events: TickEvents { link_closed: f & flags::LINK_CLOSED != 0 } })
    }
}

/// A [`PlayerInput`]'s events as a tick's flags.
pub mod flags {
    /// `TickEvents::link_closed`.
    pub const LINK_CLOSED: u8 = 1 << 0;
    pub const ALL: u8 = LINK_CLOSED;
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
/// ([`crate::world::BattleWorld::with_observer`]), which tells it every
/// tick simulated, every rewind and every tick settled; the host reads its
/// actions through the session (`Session::world`).
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

    fn confirmed(&mut self, frame: u32, _settled: Option<&Battle>) {
        self.tracker.confirmed(frame + 1);
        if self.unconfirmed.front().is_some_and(|&(f, _)| f == frame) {
            let (frame, cues) = self.unconfirmed.pop_front().unwrap();
            self.confirmed.extend(cues.into_iter().map(|c| (frame, c)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Delivery, InputLink};
    use nettai_battle::input::keys;

    /// A player's input goes through a link and comes out the same: the
    /// buttons, and the link closing.
    #[test]
    fn a_player_input_crosses_the_link() {
        let inputs = [
            PlayerInput { tick: PlayerTick { held: keys::A | keys::LEFT }, events: TickEvents::default() },
            PlayerInput { tick: PlayerTick { held: 0 }, events: TickEvents { link_closed: true } },
            PlayerInput { tick: PlayerTick { held: keys::L }, events: TickEvents::default() },
        ];
        let (mut a, mut b) = (InputLink::<PlayerInput>::new(64), InputLink::<PlayerInput>::new(64));
        for (i, input) in inputs.iter().enumerate() {
            a.push(input, i as i16);
        }
        let datagram = a.datagram(0);
        let mut delivered = Vec::new();
        b.receive(&datagram, 0, &mut delivered).unwrap();
        let got: Vec<PlayerInput> = delivered
            .into_iter()
            .map(|d| match d {
                Delivery::Input { input, .. } => input,
                d => panic!("{d:?}"),
            })
            .collect();
        assert_eq!(got, inputs);
        // A frame's header, then a byte or two a tick.
        assert!(datagram.len() < 16, "{} bytes", datagram.len());
        // A flag no event has is refused.
        assert!(PlayerInput::decode(0, 2).is_err());
    }
}
