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

/// A player's input on the wire: the tick's word (the buttons, the switch
/// request, the microphone heard or not and the custom screen's choice), and the
/// events as flags
/// ([`TickFlags`]).
impl WireInput for PlayerInput {
    fn encode(&self) -> (u32, u8) {
        let TickEvents { link_closed } = &self.events;
        let mut flags = TickFlags::empty();
        flags.set(TickFlags::LINK_CLOSED, *link_closed);
        (self.tick.word(), flags.bits())
    }

    fn decode(word: u32, f: u8) -> io::Result<PlayerInput> {
        let Some(flags) = TickFlags::from_bits(f) else {
            return Err(crate::protocol::invalid("unknown event flags"));
        };
        Ok(PlayerInput { tick: PlayerTick::from_word(word), events: TickEvents { link_closed: flags.contains(TickFlags::LINK_CLOSED) } })
    }
}

bitflags::bitflags! {
    /// A [`PlayerInput`]'s events as a tick's flags on the wire. A bit none
    /// of them names is refused (`decode`).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TickFlags: u8 {
        /// `TickEvents::link_closed`.
        const LINK_CLOSED = 1 << 0;
    }
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

    /// Buttons (and the microphone) carry on; events, a switch
    /// request and a custom screen's choice happen once.
    fn predict(last: &PlayerInput) -> PlayerInput {
        PlayerInput { tick: PlayerTick { switch_to: None, choice: None, ..last.tick }, events: TickEvents::default() }
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

    fn confirmed(&mut self, frame: u32, _: [&G::Input; 2], _settled: Option<&Battle>) {
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
    use nettai_battle::input::Keys;

    /// A player's input goes through a link and comes out the same: the
    /// buttons, a custom screen's choice, and the link closing.
    #[test]
    fn a_player_input_crosses_the_link() {
        let inputs = [
            PlayerInput { tick: PlayerTick::buttons(Keys::A | Keys::LEFT), events: TickEvents::default() },
            PlayerInput { tick: PlayerTick::buttons(Keys::empty()), events: TickEvents { link_closed: true } },
            PlayerInput { tick: PlayerTick::buttons(Keys::L), events: TickEvents::default() },
            PlayerInput { tick: PlayerTick::buttons(Keys::A).with_choice(Some(nettai_battle::ScreenChoice::Pick(2))), events: TickEvents::default() },
            PlayerInput { tick: PlayerTick::buttons(Keys::empty()).with_choice(Some(nettai_battle::ScreenChoice::Unpick(4))), events: TickEvents::default() },
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
        // A frame's header, then a byte or two a tick (three with a choice).
        assert!(datagram.len() < 22, "{} bytes", datagram.len());
        // A flag no event has is refused.
        assert!(PlayerInput::decode(0, 2).is_err());
    }

    /// A predicted input carries the buttons on, and nothing that happens
    /// once: no event, switch request or choice.
    #[test]
    fn a_prediction_carries_the_buttons_alone() {
        let tick = PlayerTick {
            held: Keys::B,
            switch_to: Some(2),
            mic_loud: true,
            choice: Some(nettai_battle::ScreenChoice::Ok),
        };
        let last = PlayerInput { tick, events: TickEvents { link_closed: true } };
        let p = <Battle as Game>::predict(&last);
        assert_eq!(p, PlayerInput { tick: PlayerTick { mic_loud: true, ..PlayerTick::buttons(Keys::B) }, events: TickEvents::default() });
    }
}
