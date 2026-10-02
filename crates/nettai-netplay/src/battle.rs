//! The battle engine as a rollback [`Game`].
//!
//! [`Battle`] steps on the engine's per-tick input record
//! ([`TickInput`]). Each player contributes their share of it: their
//! buttons, and the frame's events their console produced ([`PlayerInput`]):
//! the link session closing at the end of a round, and, only when checking
//! against a recording that lacks a player's folder, that player's
//! recorded custom-screen results. Carrying the events in the inputs is
//! what makes both peers step each frame with the same ones. On the wire
//! (`WireInput`), the events are a tick's flags, and a recorded result the
//! tick's payload.
//!
//! Both peers simulate from the same perspective (`RoundSetup::local_side`
//! is part of the shared setup); each presents it for its own player
//! (`Battle::sound_cues_for`, `Battle::banner_for`, ...), see
//! docs/design/rollback.md.

use std::collections::VecDeque;
use std::io;

use crate::protocol::WireInput;
use crate::wire;
use crate::world::{Game, Observer};
use nettai_battle::cues::{CueAction, CueId, CueTracker};
use nettai_battle::custom::Recorded;
use nettai_battle::{Battle, CustomResult, PlayerTick, SoundCue, TickEvents, TickInput};

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

/// A player's input on the wire: the buttons, the events as flags
/// ([`flags`]), and a recorded result's bytes as the payload (only when
/// checking against a recording, once a custom screen).
impl WireInput for PlayerInput {
    fn encode(&self, payload: &mut Vec<u8>) -> (u16, u8) {
        let TickEvents { link_closed, recorded } = &self.events;
        let mut f = if *link_closed { flags::LINK_CLOSED } else { 0 };
        for (side, r) in recorded.iter().enumerate() {
            let Some(Recorded { in_custom, result }) = r else { continue };
            f |= flags::RECORDED[side];
            if *in_custom {
                f |= flags::IN_CUSTOM[side];
            }
            if let Some(result) = result {
                f |= flags::RESULT[side];
                wire::Writer(payload).put(&**result);
            }
        }
        (self.tick.held, f)
    }

    fn decode(held: u16, f: u8, payload: &[u8]) -> io::Result<PlayerInput> {
        if f & !flags::ALL != 0 {
            return Err(crate::protocol::invalid("unknown event flags"));
        }
        let mut bytes = wire::Reader::new(payload);
        let mut events = TickEvents { link_closed: f & flags::LINK_CLOSED != 0, ..TickEvents::default() };
        for side in 0..2 {
            let result = if f & flags::RESULT[side] != 0 { Some(Box::new(bytes.get::<CustomResult>()?)) } else { None };
            if f & flags::RECORDED[side] != 0 {
                events.recorded[side] = Some(Recorded { in_custom: f & flags::IN_CUSTOM[side] != 0, result });
            } else if result.is_some() || f & flags::IN_CUSTOM[side] != 0 {
                return Err(crate::protocol::invalid("a recorded result without its record"));
            }
        }
        bytes.finish()?;
        Ok(PlayerInput { tick: PlayerTick { held }, events })
    }
}

/// A [`PlayerInput`]'s events as a tick's flags.
pub mod flags {
    /// `TickEvents::link_closed`.
    pub const LINK_CLOSED: u8 = 1 << 0;
    /// `TickEvents::recorded[side]` is there.
    pub const RECORDED: [u8; 2] = [1 << 1, 1 << 2];
    /// ... and says the side's custom screen is open.
    pub const IN_CUSTOM: [u8; 2] = [1 << 3, 1 << 4];
    /// ... and carries a result (in the tick's payload, side 0's first).
    pub const RESULT: [u8; 2] = [1 << 5, 1 << 6];
    pub const ALL: u8 = 0x7F;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{Delivery, InputLink};
    use nettai_battle::content::testing;
    use nettai_battle::input::keys;

    /// A player's input with every kind of event goes through a link and
    /// comes out the same: the buttons, the link closing, each side's
    /// recorded screen status and a result (the tick's payload, in chunks).
    #[test]
    fn a_player_input_crosses_the_link() {
        let result = CustomResult {
            hand: Some(nettai_battle::hand::ChipHand::empty(&testing::content())),
            navi_stats: testing::stats(500),
            transform: Default::default(),
        };
        let inputs = [
            PlayerInput { tick: PlayerTick { held: keys::A | keys::LEFT }, events: TickEvents::default() },
            PlayerInput { tick: PlayerTick { held: 0 }, events: TickEvents { link_closed: true, ..TickEvents::default() } },
            PlayerInput {
                tick: PlayerTick { held: keys::L },
                events: TickEvents {
                    link_closed: false,
                    recorded: [Some(Recorded { in_custom: true, result: None }), Some(Recorded { in_custom: false, result: Some(Box::new(result)) })],
                },
            },
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
        // The plain ticks are a byte each; the result is a few chunks.
        assert!(datagram.len() < 200, "{} bytes", datagram.len());
    }
}
