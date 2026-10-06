//! One peer of a match: a getgud session on its player's battle, the
//! rennet link to the other peer, and what a host does every frame
//! (docs/design/rollback.md §4.2):
//!
//! 1. [`Peer::receive`] every datagram that arrived;
//! 2. [`Peer::wait`]: hold this frame if the peer runs ahead (clock sync)
//!    or has too many unconfirmed inputs (the stall guard);
//! 3. if not, [`Peer::decide`] its player's input for the next tick;
//! 4. send [`Peer::datagram`] (every frame, waiting or not: it carries the
//!    acks and the redundancy window);
//! 5. if it decided, [`Peer::advance`] and draw the presented frame.
//!
//! The simulator (`sim`) and a frontend's host loop both drive a match
//! this way. The peer is pure: datagrams in and out, the time passed in.
//! Where the players' input ends at a tick both peers know (a replay's
//! last, the simulator's `max_frames`), a peer [`Peer::drain`]s instead of
//! deciding and advancing, until its settled state reaches the end.
//!
//! What the peer's world tells its observer (every tick simulated, every
//! rewind, every tick settled) needs nothing from the host: the host reads
//! the observer through [`Peer::session`] (`Session::world`). A peer is
//! `Send` when its observer is, so it can run on a network thread.
//!
//! Rounds: a set's rounds are one session each, on one stream. When the
//! host has finished a round (its settled state is over), it starts the
//! next with [`Peer::end_round`], which marks the stream; the other
//! player's inputs after their mark are the next round's, and those before
//! it that went past the end are dropped.

use std::collections::VecDeque;

use getgud::{Advance, Confirmed, Session};

use crate::link::{Delivery, InputLink, LinkError, LinkStats};
use crate::protocol::{self, WireInput};
use crate::world::{BattleWorld, Game, Observer};

/// Clock sync: how much skew a peer that runs ahead adds up before it
/// stalls a frame. It slows down by `skew / SKEW_PER_STALL` frames a
/// frame, as a frame-rate adjustment would (a frame a second per tick of
/// skew, at 60 fps). A peer one frame ahead of the other shows a skew of
/// 2 (it leads by one more, the other by one less). Stalling on any
/// positive skew at once over-corrects: a stall shows in the skew in full
/// only a round trip later, and jitter makes the skew noisy, so the peers
/// would take turns stalling.
pub const SKEW_PER_STALL: i32 = 60;

/// How a peer plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerConfig {
    /// Ticks the peer presents behind its newest local input (getgud's
    /// present delay): the player's own, never sent to the other peer,
    /// and changeable during a match ([`Peer::set_present_delay`]). More
    /// spares that much prediction (fewer rollbacks shown) and shows the
    /// player's input that much later; 0 shows the newest tick.
    pub present_delay: u32,
    /// The stall guard: a peer with this many local inputs that no remote
    /// input matches yet waits, unless remote input it has can be matched
    /// (getgud's `matchable`). It bounds how deep a peer speculates.
    pub max_lead: u32,
    /// The rollback horizon of the link ([`protocol::HORIZON`]).
    pub horizon: u32,
}

impl PeerConfig {
    /// `max_lead` and the full horizon; the stall guard must fit it
    /// ([`protocol::max_lead`]).
    pub fn new(present_delay: u32, max_lead: u32) -> PeerConfig {
        let config = PeerConfig { present_delay, max_lead, horizon: protocol::HORIZON };
        assert!(config.fits(), "a stall guard of {max_lead} doesn't fit the horizon");
        config
    }

    /// The stall guard fits the horizon: two peers that both keep it never
    /// open a gap past it. (A peer that doesn't keep it, or a horizon that
    /// is too small, tears the match down: see `sim`'s tests.)
    pub fn fits(&self) -> bool {
        self.max_lead <= protocol::max_lead(self.horizon)
    }
}

/// Why a peer holds a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    /// The stall guard: too many of its inputs are unconfirmed.
    Parked,
    /// Clock sync: it runs ahead of the other peer.
    Stalled,
}

/// What a peer did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerStats {
    /// Ticks simulated for the first time (the simulator counts them).
    pub simulated: u64,
    /// Rollbacks: advances that found a misprediction and threw the
    /// speculative ticks from it away (getgud's `last_misprediction_depth`,
    /// when it isn't 0).
    pub rollbacks: u64,
    /// The ticks those rollbacks threw away, which are simulated again.
    pub resimulated: u64,
    /// The deepest rollback, in ticks, and the latest.
    pub max_rollback: u32,
    pub last_rollback: u32,
    /// The furthest a presented frame ran past the confirmed input, in
    /// ticks (getgud's `speculation_balance`).
    pub max_speculation: u32,
    /// The same, added up over every advance.
    pub speculated: u64,
    /// Advances (frames presented).
    pub advances: u64,
    /// Wall frames stalled for clock sync: the peer ran ahead.
    pub stalls: u64,
    /// Wall frames the stall guard held the peer.
    pub parked: u64,
    /// Rows settled by promoting their speculation (the simulator counts
    /// them; the rest were simulated as they settled).
    pub promoted: u64,
}

impl PeerStats {
    /// How far a presented frame ran past the confirmed input, on average.
    pub fn mean_speculation(&self) -> f64 {
        self.speculated as f64 / self.advances.max(1) as f64
    }
}

/// One peer: its player's session and its end of the link.
pub struct Peer<G: Game, O: Observer<G> = ()>
where
    G::Input: WireInput,
{
    session: Session<BattleWorld<G, O>>,
    link: InputLink<G::Input>,
    config: PeerConfig,
    /// Clock sync: the skew added up, and whether the last frame stalled.
    drift: i32,
    stalled: bool,
    /// Rounds this peer finished, and the other player's (their marks).
    round: u32,
    remote_round: u32,
    /// The other player's inputs for the round after this peer's, until it
    /// starts.
    early: VecDeque<(G::Input, i16)>,
    remote_left: bool,
    /// The input decided for the next advance.
    decided: Option<G::Input>,
    delivered: Vec<Delivery<G::Input>>,
    stats: PeerStats,
}

impl<G: Game, O: Observer<G>> Peer<G, O>
where
    G::Input: WireInput,
{
    /// The peer playing `world`'s side, from its start.
    pub fn new(world: BattleWorld<G, O>, config: PeerConfig) -> Peer<G, O> {
        Peer {
            session: world.session(config.present_delay),
            link: InputLink::new(config.horizon),
            config,
            drift: 0,
            stalled: false,
            round: 0,
            remote_round: 0,
            early: VecDeque::new(),
            remote_left: false,
            decided: None,
            delivered: Vec::new(),
            stats: PeerStats::default(),
        }
    }

    pub fn session(&self) -> &Session<BattleWorld<G, O>> {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session<BattleWorld<G, O>> {
        &mut self.session
    }

    pub fn config(&self) -> PeerConfig {
        self.config
    }

    pub fn stats(&self) -> &PeerStats {
        &self.stats
    }

    pub fn link_stats(&self) -> &LinkStats {
        self.link.stats()
    }

    pub fn link(&self) -> &InputLink<G::Input> {
        &self.link
    }

    /// Rounds this peer finished.
    pub fn round(&self) -> u32 {
        self.round
    }

    /// Rounds the other player finished.
    pub fn remote_round(&self) -> u32 {
        self.remote_round
    }

    /// The other player left the match.
    pub fn remote_left(&self) -> bool {
        self.remote_left
    }

    /// Take a datagram from the other peer, received at `now`. An error
    /// ends the match: a malformed datagram (the other isn't a peer of this
    /// protocol) or a gap past the horizon.
    pub fn receive(&mut self, datagram: &[u8], now: u64) -> Result<(), LinkError> {
        let mut delivered = std::mem::take(&mut self.delivered);
        let mut result = self.link.receive(datagram, now, &mut delivered);
        for d in delivered.drain(..) {
            match d {
                Delivery::Input { input, tick_advantage } => {
                    if self.remote_round == self.round {
                        self.session.add_remote_input(0, input, tick_advantage);
                    } else if self.remote_round > self.round {
                        self.early.push_back((input, tick_advantage));
                    }
                    // (Behind: their input past the end of a round this
                    // peer has finished.)
                }
                Delivery::RoundEnd => {
                    self.remote_round += 1;
                    if self.remote_round > self.round + 1 && result.is_ok() {
                        result = Err(LinkError::Malformed(protocol::invalid("the other player ended a round this one hasn't started")));
                    }
                }
                Delivery::MatchEnd => self.remote_left = true,
            }
        }
        self.delivered = delivered;
        result
    }

    /// Whether this frame waits, and why: the stall guard (too many local
    /// inputs unconfirmed and none of the other's to match them), or clock
    /// sync. Clock sync adds the skew up every frame while the presented
    /// frame speculates (until then the present delay absorbs the lead) and
    /// stalls a frame for every [`SKEW_PER_STALL`] of it, never two in a
    /// row, so that the peer keeps sending its advantage and the two can't
    /// wait on each other.
    pub fn wait(&mut self) -> Option<Wait> {
        let s = &self.session;
        if s.local_queue_length() >= self.config.max_lead as usize && s.matchable() == 0 {
            self.stats.parked += 1;
            return Some(Wait::Parked);
        }
        if s.speculation_balance() >= 0 {
            self.drift = (self.drift + s.skew()).max(0);
        }
        if self.drift >= SKEW_PER_STALL && !self.stalled {
            self.drift -= SKEW_PER_STALL;
            self.stalled = true;
            self.stats.stalls += 1;
            return Some(Wait::Stalled);
        }
        self.stalled = false;
        None
    }

    /// The next tick's local input (`local_frontier`): sent with this
    /// frame's datagram, simulated by the next [`Peer::advance`].
    pub fn decide(&mut self, input: G::Input) {
        assert!(self.decided.is_none(), "an input is decided already");
        self.link.push(&input, self.session.local_tick_advantage());
        self.decided = Some(input);
    }

    /// The datagram to send this frame.
    pub fn datagram(&mut self, now: u64) -> Vec<u8> {
        self.link.datagram(now)
    }

    /// Advance the session with the decided input, and look at the result
    /// (the presented frame, the rows that settled).
    pub fn advance<R>(&mut self, look: impl FnOnce(Advance<'_, BattleWorld<G, O>>) -> R) -> R {
        let input = self.decided.take().expect("advance after decide");
        let r = match self.session.advance(input) {
            Ok(advanced) => look(advanced),
            Err(e) => match e {},
        };
        let s = &self.session;
        let depth = s.last_misprediction_depth();
        self.stats.last_rollback = depth;
        if depth > 0 {
            self.stats.rollbacks += 1;
            self.stats.resimulated += depth as u64;
            self.stats.max_rollback = self.stats.max_rollback.max(depth);
        }
        let speculation = s.speculation_balance().max(0) as u32;
        self.stats.max_speculation = self.stats.max_speculation.max(speculation);
        self.stats.speculated += speculation as u64;
        self.stats.advances += 1;
        r
    }

    /// The player's input has ended (at a tick both peers know): settle
    /// what the other player's last inputs confirm, without new input and
    /// speculating nothing past the end (getgud's `drain`), and look at the
    /// rows that settled. Call it each frame instead of deciding and
    /// advancing, until `session().is_drained()`; present the settled state.
    pub fn drain<R>(&mut self, look: impl FnOnce(Vec<Confirmed<'_, BattleWorld<G, O>>>) -> R) -> R {
        assert!(self.decided.is_none(), "drain instead of deciding an input");
        let r = match self.session.drain() {
            Ok(rows) => look(rows),
            Err(e) => match e {},
        };
        let depth = self.session.last_misprediction_depth();
        self.stats.last_rollback = depth;
        if depth > 0 {
            self.stats.rollbacks += 1;
            self.stats.resimulated += depth as u64;
            self.stats.max_rollback = self.stats.max_rollback.max(depth);
        }
        r
    }

    /// Present `ticks` behind the newest local input from the next frame on
    /// (getgud's `Session::set_present_delay`), this round and the rounds
    /// after.
    pub fn set_present_delay(&mut self, ticks: u32) {
        self.config.present_delay = ticks;
        self.session.set_present_delay(ticks);
    }

    /// This peer's round is over (its settled state is): mark the stream and
    /// play the next round on `next`, a world at its start. (An observer
    /// that goes on from round to round, the sound, the host takes from this
    /// round's world first: `session_mut().world_mut().observer_mut()`.)
    pub fn end_round(&mut self, next: BattleWorld<G, O>) {
        assert!(self.decided.is_none(), "a round ends between frames");
        self.link.push_round_end();
        self.round += 1;
        self.session = next.session(self.config.present_delay);
        self.drift = 0;
        self.stalled = false;
        if self.remote_round == self.round {
            for (input, advantage) in self.early.drain(..) {
                self.session.add_remote_input(0, input, advantage);
            }
        }
    }

    /// This player leaves: the next datagram tells the other peer.
    pub fn leave(&mut self) {
        self.link.push_match_end();
    }
}
