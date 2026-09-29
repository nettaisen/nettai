//! Two peers over simulated links, in one process: the rollback
//! simulator the tests drive.
//!
//! Each wall-clock frame, every peer decides its player's input
//! `input_delay` frames ahead and sends it; then every peer receives the
//! packets that have arrived and updates (rolling back if a prediction
//! was wrong). With no latency a packet arrives in the frame it was sent. Every frame
//! either peer confirms is checked: both peers' digests of the confirmed
//! state, and the digest of a plain lockstep run of the same inputs, must
//! be equal.

use crate::network::{Link, LinkConfig};
use crate::peer::{Observer, Peer, PeerStats};
use crate::Game;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// How the peers are connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetConfig {
    /// Both directions.
    pub link: LinkConfig,
    /// Frames between deciding a local input and the frame it's for.
    pub input_delay: u32,
    /// How far a peer runs past the remote input it has.
    pub max_prediction: u32,
    /// Seeds the links' jitter.
    pub seed: u64,
}

impl NetConfig {
    /// `latency` frames each way, up to `jitter` more, no input delay.
    pub fn latency(latency: u32, jitter: u32) -> NetConfig {
        NetConfig {
            link: LinkConfig { latency, jitter },
            input_delay: 0,
            max_prediction: (latency + jitter + 2).max(8),
            seed: 0x5EED ^ (latency as u64) << 8 ^ jitter as u64,
        }
    }
}

/// Where the peers disagreed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divergence {
    pub frame: u32,
    /// The two peers' digests of the state after `frame`.
    pub peers: [u64; 2],
    /// The lockstep run's.
    pub lockstep: u64,
}

/// A peer's simulation panicked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panicked {
    pub peer: usize,
    /// The frame being simulated.
    pub frame: u32,
    /// The lockstep run gets through that frame: the panic happened on
    /// predicted input only.
    pub speculative: bool,
    pub message: String,
}

/// How a match went.
#[derive(Clone, Debug)]
pub struct Report {
    /// Frames both peers confirmed.
    pub frames: u32,
    pub wall_frames: u64,
    pub peers: [PeerStats; 2],
    /// Both peers' confirmed states are over.
    pub over: bool,
    pub divergence: Option<Divergence>,
    pub panicked: Option<Panicked>,
}

impl Report {
    /// Ran to the end (or to the frame limit) with the peers in sync.
    pub fn in_sync(&self) -> bool {
        self.divergence.is_none() && self.panicked.is_none()
    }
}

/// Records a peer's confirmed digests and last simulated frame, and
/// passes everything on.
struct Tap<'a, O> {
    inner: &'a mut O,
    digests: &'a mut Vec<u64>,
    simulating: &'a mut u32,
}

impl<G: Game, O: Observer<G>> Observer<G> for Tap<'_, O> {
    fn rolled_back(&mut self, frame: u32) {
        *self.simulating = frame;
        self.inner.rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G, resim: bool) {
        *self.simulating = frame + 1;
        self.inner.simulated(frame, game, resim);
    }
    fn confirmed(&mut self, frame: u32, game: &G) {
        debug_assert_eq!(self.digests.len(), frame as usize);
        self.digests.push(game.digest());
        self.inner.confirmed(frame, game);
    }
}

/// Two peers, their links, and the lockstep reference.
pub struct Match<G: Game> {
    pub config: NetConfig,
    peers: [Peer<G>; 2],
    /// links[p]: from peer p to the other.
    links: [Link<(u32, G::Input)>; 2],
    /// Each player's inputs as decided, by frame.
    decided: [Vec<G::Input>; 2],
    lockstep: G,
    lockstep_frames: u32,
    lockstep_digests: Vec<u64>,
    digests: [Vec<u64>; 2],
    simulating: [u32; 2],
    checked: u32,
    now: u64,
    divergence: Option<Divergence>,
    panicked: Option<Panicked>,
}

impl<G: Game> Match<G> {
    /// Both peers start from `start`; peer `p` controls player `p`.
    pub fn new(start: &G, config: NetConfig) -> Match<G> {
        Match::from_starts([start.clone(), start.clone()], start.clone(), config)
    }

    /// Each peer and the lockstep run start from their own state (equal,
    /// unless a test wants otherwise).
    pub fn from_starts(peers: [G; 2], lockstep: G, config: NetConfig) -> Match<G> {
        let link = |p: u64| Link::new(config.link, config.seed.wrapping_mul(0x9E37_79B9).wrapping_add(p));
        let [a, b] = peers;
        Match {
            config,
            peers: [Peer::new(0, a, config.max_prediction), Peer::new(1, b, config.max_prediction)],
            links: [link(0), link(1)],
            decided: [Vec::new(), Vec::new()],
            lockstep,
            lockstep_frames: 0,
            lockstep_digests: Vec::new(),
            digests: [Vec::new(), Vec::new()],
            simulating: [0, 0],
            checked: 0,
            now: 0,
            divergence: None,
            panicked: None,
        }
    }

    pub fn peer(&self, p: usize) -> &Peer<G> {
        &self.peers[p]
    }

    /// Peer `p`'s digest of every confirmed frame's state, by frame.
    pub fn confirmed_digests(&self, p: usize) -> &[u64] {
        &self.digests[p]
    }

    /// Frames both peers have confirmed.
    pub fn confirmed(&self) -> u32 {
        self.peers[0].confirmed().min(self.peers[1].confirmed())
    }

    /// One wall-clock frame. `inputs(player, frame)` decides a player's
    /// input; it is called once per frame and player, in frame order.
    /// `observers[p]` sees what peer `p` simulates. Returns false once the
    /// match can't go on (divergence or panic).
    pub fn step<O: Observer<G>>(&mut self, inputs: &mut impl FnMut(usize, u32) -> G::Input, observers: &mut [O; 2]) -> bool {
        if !self.in_sync() {
            return false;
        }
        for p in 0..2 {
            let horizon = self.peers[p].frame() + self.config.input_delay;
            while (self.decided[p].len() as u32) <= horizon {
                let frame = self.decided[p].len() as u32;
                let input = inputs(p, frame);
                self.decided[p].push(input.clone());
                self.peers[p].add_local_input(frame, input.clone());
                self.links[p].send(self.now, (frame, input));
            }
        }
        for (p, observer) in observers.iter_mut().enumerate() {
            for (frame, input) in self.links[1 - p].receive(self.now) {
                self.peers[p].add_remote_input(frame, input);
            }
            let peer = &mut self.peers[p];
            let mut tap = Tap { inner: observer, digests: &mut self.digests[p], simulating: &mut self.simulating[p] };
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| peer.update(&mut tap))) {
                let frame = self.simulating[p];
                let message = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                let speculative = self.lockstep_passes(frame);
                self.panicked = Some(Panicked { peer: p, frame, speculative, message });
                return false;
            }
        }
        self.advance_lockstep();
        self.check();
        self.now += 1;
        self.in_sync()
    }

    fn in_sync(&self) -> bool {
        self.divergence.is_none() && self.panicked.is_none()
    }

    fn advance_lockstep(&mut self) {
        let decided = self.decided[0].len().min(self.decided[1].len()) as u32;
        let target = decided.min(self.confirmed());
        while self.lockstep_frames < target {
            let f = self.lockstep_frames as usize;
            let inputs = [self.decided[0][f].clone(), self.decided[1][f].clone()];
            self.lockstep.advance(&inputs);
            self.lockstep_digests.push(self.lockstep.digest());
            self.lockstep_frames += 1;
        }
    }

    /// Whether the lockstep run gets through `frame` (catching a panic).
    fn lockstep_passes(&mut self, frame: u32) -> bool {
        let decided = self.decided[0].len().min(self.decided[1].len()) as u32;
        if frame >= decided {
            return false;
        }
        let mut g = self.lockstep.clone();
        let from = self.lockstep_frames;
        catch_unwind(AssertUnwindSafe(|| {
            for f in from..=frame {
                g.advance(&[self.decided[0][f as usize].clone(), self.decided[1][f as usize].clone()]);
            }
        }))
        .is_ok()
    }

    fn check(&mut self) {
        let limit = self.confirmed().min(self.lockstep_frames);
        while self.checked < limit {
            let f = self.checked as usize;
            let peers = [self.digests[0][f], self.digests[1][f]];
            let lockstep = self.lockstep_digests[f];
            if peers[0] != peers[1] || peers[0] != lockstep {
                self.divergence = Some(Divergence { frame: self.checked, peers, lockstep });
                return;
            }
            self.checked += 1;
        }
    }

    /// Run until both peers' confirmed states are over, `max_frames` are
    /// confirmed (the peers simulate no further), or the peers fall out
    /// of sync.
    pub fn run<O: Observer<G>>(
        &mut self,
        mut inputs: impl FnMut(usize, u32) -> G::Input,
        observers: &mut [O; 2],
        max_frames: u32,
    ) -> Report {
        for p in &mut self.peers {
            p.set_end(max_frames);
        }
        let wall_limit = max_frames as u64 * 4 + 1000;
        while self.now < wall_limit {
            let over = self.peers.iter().all(|p| p.confirmed_game().is_over());
            if over || self.confirmed() >= max_frames {
                break;
            }
            if !self.step(&mut inputs, observers) {
                break;
            }
        }
        self.report()
    }

    pub fn report(&self) -> Report {
        Report {
            frames: self.confirmed(),
            wall_frames: self.now,
            peers: [self.peers[0].stats().clone(), self.peers[1].stats().clone()],
            over: self.peers.iter().all(|p| p.confirmed_game().is_over()),
            divergence: self.divergence.clone(),
            panicked: self.panicked.clone(),
        }
    }
}
