//! One peer's rollback session.

use crate::Game;
use std::collections::VecDeque;

/// Hooks into what a peer simulates: for presentation (sound cues) and
/// for checking. Frames are numbered from 0; "the state after frame `f`"
/// is the state once `f + 1` frames have been simulated.
pub trait Observer<G: Game> {
    /// Frames `frame` and later are about to be simulated again.
    fn rolled_back(&mut self, _frame: u32) {}
    /// Frame `frame` was simulated (`resim`: again, after a rollback).
    /// `game` is the state after it.
    fn simulated(&mut self, _frame: u32, _game: &G, _resim: bool) {}
    /// Frame `frame` is confirmed: both inputs are known and it was
    /// simulated with them. `game` is the state after it. Called once per
    /// frame, in order.
    fn confirmed(&mut self, _frame: u32, _game: &G) {}
}

impl<G: Game> Observer<G> for () {}

/// What a peer did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerStats {
    /// Frames simulated for the first time.
    pub simulated: u64,
    /// ... of which with a predicted remote input.
    pub predicted: u64,
    /// Rollbacks (a remote input differed from its prediction).
    pub rollbacks: u64,
    /// Frames simulated again after rollbacks.
    pub resimulated: u64,
    /// The deepest rollback, in frames.
    pub max_rollback: u32,
    /// Updates that could not advance (too far ahead of the remote input,
    /// or the local input not decided yet).
    pub stalls: u64,
}

/// One peer: the simulation, both players' inputs as far as they are
/// known, and a snapshot of every unconfirmed frame.
pub struct Peer<G: Game> {
    player: usize,
    game: G,
    /// Frames simulated: `game` is the state after `frame` frames.
    frame: u32,
    /// Frames confirmed.
    confirmed: u32,
    /// Each player's inputs by frame, as far as known.
    inputs: [Vec<Option<G::Input>>; 2],
    /// Frames `0..known[p]` of player `p` are all known.
    known: [u32; 2],
    /// The inputs frames `confirmed..frame` were simulated with.
    used: VecDeque<[G::Input; 2]>,
    /// The states after `confirmed..=frame` frames.
    snapshots: VecDeque<G>,
    /// The earliest simulated frame whose remote input turned out wrong.
    first_wrong: Option<u32>,
    max_prediction: u32,
    /// No frame from this one on is simulated.
    end: u32,
    stats: PeerStats,
}

impl<G: Game> Peer<G> {
    /// A peer controlling `player` (0 or 1), starting from `game`, that
    /// runs at most `max_prediction` frames past the remote input it has.
    pub fn new(player: usize, game: G, max_prediction: u32) -> Peer<G> {
        assert!(player < 2 && max_prediction > 0);
        Peer {
            player,
            snapshots: VecDeque::from([game.clone()]),
            game,
            frame: 0,
            confirmed: 0,
            inputs: [Vec::new(), Vec::new()],
            known: [0, 0],
            used: VecDeque::new(),
            first_wrong: None,
            max_prediction,
            end: u32::MAX,
            stats: PeerStats::default(),
        }
    }

    pub fn player(&self) -> usize {
        self.player
    }

    /// Frames simulated (the next frame to simulate).
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Frames confirmed.
    pub fn confirmed(&self) -> u32 {
        self.confirmed
    }

    /// The newest state (after `frame()` frames; predicted past
    /// `confirmed()`). This is what a frontend presents.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// The state after the confirmed frames.
    pub fn confirmed_game(&self) -> &G {
        &self.snapshots[0]
    }

    pub fn stats(&self) -> &PeerStats {
        &self.stats
    }

    /// Simulate no further than `frames` frames (the end of the input).
    pub fn set_end(&mut self, frames: u32) {
        self.end = frames;
    }

    fn known_input(&self, p: usize, frame: u32) -> Option<&G::Input> {
        self.inputs[p].get(frame as usize).and_then(Option::as_ref)
    }

    fn set(&mut self, p: usize, frame: u32, input: G::Input) {
        let v = &mut self.inputs[p];
        if v.len() <= frame as usize {
            v.resize(frame as usize + 1, None);
        }
        v[frame as usize] = Some(input);
        while self.known_input(p, self.known[p]).is_some() {
            self.known[p] += 1;
        }
    }

    /// This peer's player's input for `frame`, decided locally (for the
    /// next frame to simulate or later).
    pub fn add_local_input(&mut self, frame: u32, input: G::Input) {
        assert!(frame >= self.frame, "local input for frame {frame}, already simulated");
        assert!(self.known_input(self.player, frame).is_none(), "local input for frame {frame} given twice");
        self.set(self.player, frame, input);
    }

    /// The other player's input for `frame`, as it arrived. Duplicates are
    /// ignored; one that differs from what an already simulated frame
    /// assumed schedules a rollback for the next update.
    pub fn add_remote_input(&mut self, frame: u32, input: G::Input) {
        let p = 1 - self.player;
        if frame < self.confirmed || self.known_input(p, frame).is_some() {
            return;
        }
        if frame < self.frame && self.used[(frame - self.confirmed) as usize][p] != input {
            self.first_wrong = Some(self.first_wrong.map_or(frame, |f| f.min(frame)));
        }
        self.set(p, frame, input);
    }

    /// Player `p`'s input for `frame`: known, or predicted from the latest
    /// known one before it.
    fn input_for(&self, p: usize, frame: u32) -> (G::Input, bool) {
        if let Some(i) = self.known_input(p, frame) {
            return (i.clone(), false);
        }
        let latest = (0..frame).rev().take_while(|&f| f + 1 >= self.known[p]).find_map(|f| self.known_input(p, f));
        (latest.map(G::predict).unwrap_or_else(G::blank_input), true)
    }

    /// Whether the next frame can be simulated: this peer's input for it
    /// is decided, and it is within `max_prediction` frames of the remote
    /// input.
    pub fn can_advance(&self) -> bool {
        self.frame < self.end
            && self.known_input(self.player, self.frame).is_some()
            && self.frame < self.known[1 - self.player] + self.max_prediction
    }

    /// One update: roll back and simulate again if a prediction was wrong,
    /// simulate the next frame if possible, then confirm what can be.
    /// Returns whether a new frame was simulated.
    pub fn update(&mut self, obs: &mut impl Observer<G>) -> bool {
        if let Some(wrong) = self.first_wrong.take() {
            let target = self.frame;
            let back = (wrong - self.confirmed) as usize;
            self.game.clone_from(&self.snapshots[back]);
            self.snapshots.truncate(back + 1);
            self.used.truncate(back);
            self.frame = wrong;
            let depth = target - wrong;
            self.stats.rollbacks += 1;
            self.stats.resimulated += depth as u64;
            self.stats.max_rollback = self.stats.max_rollback.max(depth);
            obs.rolled_back(wrong);
            while self.frame < target {
                self.simulate(obs, true);
            }
        }
        let advance = self.can_advance();
        if advance {
            self.simulate(obs, false);
        } else {
            self.stats.stalls += 1;
        }
        self.confirm(obs);
        advance
    }

    fn simulate(&mut self, obs: &mut impl Observer<G>, resim: bool) {
        let f = self.frame;
        let (a, _) = self.input_for(0, f);
        let (b, _) = self.input_for(1, f);
        if !resim {
            self.stats.simulated += 1;
            if self.known_input(1 - self.player, f).is_none() {
                self.stats.predicted += 1;
            }
        }
        let inputs = [a, b];
        self.game.advance(&inputs);
        self.used.push_back(inputs);
        self.frame += 1;
        self.snapshots.push_back(self.game.clone());
        obs.simulated(f, &self.game, resim);
    }

    fn confirm(&mut self, obs: &mut impl Observer<G>) {
        let limit = self.frame.min(self.known[0]).min(self.known[1]);
        while self.confirmed < limit {
            let f = self.confirmed;
            // A wrong prediction would have been rolled back already.
            debug_assert!((0..2).all(|p| Some(&self.used[0][p]) == self.known_input(p, f)));
            self.used.pop_front();
            self.snapshots.pop_front();
            self.confirmed += 1;
            obs.confirmed(f, &self.snapshots[0]);
        }
    }
}
