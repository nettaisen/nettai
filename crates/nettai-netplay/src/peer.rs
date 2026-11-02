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
//!
//! Present delays: each peer tells the other its present delay (its
//! player's input delay) with its first tick and whenever it changes
//! (`protocol::Element::Delay`), and keeps both players' as they went
//! ([`Peer::local_delay`], [`Peer::remote_delay`]): no input and no part of
//! the simulation, but what a replay needs to show each player's custom
//! screen as they saw it (nettai-replay's `Tick::delays`).

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

/// A player's present delay from one of their ticks on: their `tick`th
/// input (from 0) of the round `round` (the rounds before it, counted from
/// 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DelayMark {
    pub round: u32,
    pub tick: u32,
    pub delay: u8,
}

/// A player's present delay as of their `tick`th input of round `round`, by
/// the marks of it in order: the last at or before it; 0 before any.
fn delay_at(marks: &[DelayMark], round: u32, tick: u32) -> u8 {
    marks.iter().rev().find(|m| (m.round, m.tick) <= (round, tick)).map_or(0, |m| m.delay)
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
    /// This player's inputs decided in its round, and the other's delivered
    /// in theirs; each one's present delay as they told it, and the last
    /// this peer sent.
    local_ticks: u32,
    remote_ticks: u32,
    local_delays: Vec<DelayMark>,
    remote_delays: Vec<DelayMark>,
    announced: Option<u8>,
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
            local_ticks: 0,
            remote_ticks: 0,
            local_delays: Vec::new(),
            remote_delays: Vec::new(),
            announced: None,
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

    /// This player's present delay as of their `tick`th input (from 0) of
    /// round `round` (counted from 0: [`Peer::round`] while it is played):
    /// the ticks between the battle presented with it and the tick it lands
    /// on (a frontend's custom screen layer read the battle presented the
    /// frame before, one tick further back). Presentation, for a replay.
    pub fn local_delay(&self, round: u32, tick: u32) -> u8 {
        delay_at(&self.local_delays, round, tick)
    }

    /// The other player's present delay as of their `tick`th input of round
    /// `round`, as they told it (`protocol::Element::Delay`); 0 before they
    /// told any. Presentation, for a replay: their custom screen layer read
    /// the battle one tick more than that before the tick their word lands
    /// on.
    pub fn remote_delay(&self, round: u32, tick: u32) -> u8 {
        delay_at(&self.remote_delays, round, tick)
    }

    /// Both players' present delays as told, in order ([`DelayMark`]).
    pub fn delays(&self) -> (&[DelayMark], &[DelayMark]) {
        (&self.local_delays, &self.remote_delays)
    }

    /// Take a datagram from the other peer, received at `now`. An error
    /// ends the match: a malformed datagram (the other isn't a peer of this
    /// protocol), a gap past the horizon, or the other's inputs running
    /// more than the horizon ahead of this peer's (a peer that keeps a
    /// stall guard the horizon fits never does: one that sends faster than
    /// it plays would grow the queue, which every advance copies, without
    /// end).
    pub fn receive(&mut self, datagram: &[u8], now: u64) -> Result<(), LinkError> {
        let mut delivered = std::mem::take(&mut self.delivered);
        let mut result = self.link.receive(datagram, now, &mut delivered);
        for d in delivered.drain(..) {
            match d {
                Delivery::Input { input, tick_advantage } => {
                    self.remote_ticks += 1;
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
                    self.remote_ticks = 0;
                    if self.remote_round > self.round + 1 && result.is_ok() {
                        result = Err(LinkError::Malformed(protocol::invalid("the other player ended a round this one hasn't started")));
                    }
                }
                Delivery::MatchEnd => self.remote_left = true,
                Delivery::Delay(delay) => self.remote_delays.push(DelayMark { round: self.remote_round, tick: self.remote_ticks, delay }),
            }
        }
        self.delivered = delivered;
        if self.session.remote_queue_length(0) + self.early.len() > self.config.horizon as usize && result.is_ok() {
            result = Err(LinkError::Malformed(protocol::invalid("the other player's inputs ran more than the rollback horizon ahead")));
        }
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
    /// frame's datagram, simulated by the next [`Peer::advance`]. The first,
    /// and the first after the present delay changed, go after the delay
    /// (`protocol::Element::Delay`).
    pub fn decide(&mut self, input: G::Input) {
        assert!(self.decided.is_none(), "an input is decided already");
        let delay = self.config.present_delay.min(u8::MAX as u32) as u8;
        if self.announced != Some(delay) {
            self.link.push_delay(delay);
            self.announced = Some(delay);
            self.local_delays.push(DelayMark { round: self.round, tick: self.local_ticks, delay });
        }
        self.link.push(&input, self.session.local_tick_advantage());
        self.local_ticks += 1;
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
    /// after; the next input decided tells the other peer
    /// (`protocol::Element::Delay`).
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
        self.local_ticks = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standin::{StandInBattle, folder, netbattle};
    use nettai_battle::Battle;
    use nettai_battle::content::testing;

    fn standin() -> StandInBattle {
        let c = testing::content();
        let f = || folder(&c, &[(testing::SUN_GUN_3, 0), (testing::SUN_GUN_1, 0)]);
        StandInBattle::new(Battle::new(netbattle(&c, testing::LINK_BATTLE, 300, 0x1357, [f(), f()]), c))
    }

    /// A peer whose other side sends inputs faster than it plays (here
    /// while this one plays none) ends the match once they run more than
    /// the horizon ahead, instead of queueing them without end.
    #[test]
    fn inputs_running_past_the_horizon_end_the_match() {
        let mut peer = Peer::new(BattleWorld::new(standin(), 0), PeerConfig::new(0, 8));
        let mut rogue = InputLink::<u32>::new(protocol::HORIZON);
        let mut delivered = Vec::new();
        let mut sent = 0;
        let result = loop {
            for _ in 0..100 {
                rogue.push(&0, 0);
            }
            sent += 100;
            let r = peer.receive(&rogue.datagram(sent), sent);
            if r.is_err() || sent > 10 * protocol::HORIZON as u64 {
                break r;
            }
            rogue.receive(&peer.datagram(sent), sent, &mut delivered).unwrap();
        };
        assert!(matches!(result, Err(LinkError::Malformed(_))), "{result:?}");
        assert!(sent <= 2 * protocol::HORIZON as u64 + 100, "{sent}");
    }

    /// Each peer tells the other its present delay with its first tick and
    /// when it changes; both keep both players' by round and tick, out of
    /// the input: a delay changed between two ticks applies from the
    /// second, and carries into the next round.
    #[test]
    fn the_present_delays_go_to_the_other_peer() {
        let mut a = Peer::new(BattleWorld::new(standin(), 0), PeerConfig::new(4, 8));
        let mut b = Peer::new(BattleWorld::new(standin(), 1), PeerConfig::new(2, 8));
        let mut now = 0;
        let frame = |a: &mut Peer<StandInBattle>, b: &mut Peer<StandInBattle>, now: u64| {
            a.decide(0);
            b.decide(0);
            let (da, db) = (a.datagram(now), b.datagram(now));
            a.receive(&db, now).unwrap();
            b.receive(&da, now).unwrap();
            a.advance(|_| ());
            b.advance(|_| ());
        };
        for _ in 0..3 {
            frame(&mut a, &mut b, now);
            now += 1;
        }
        a.set_present_delay(1);
        for _ in 0..3 {
            frame(&mut a, &mut b, now);
            now += 1;
        }
        assert_eq!([0, 2, 3, 5].map(|t| b.remote_delay(0, t)), [4, 4, 1, 1]);
        assert_eq!([0, 2, 3, 5].map(|t| a.local_delay(0, t)), [4, 4, 1, 1]);
        assert_eq!((a.remote_delay(0, 0), a.remote_delay(0, 5)), (2, 2));
        assert_eq!(b.remote_delay(1, 0), 1, "it carries into the next round");
        assert_eq!(b.delays().1, [DelayMark { round: 0, tick: 0, delay: 4 }, DelayMark { round: 0, tick: 3, delay: 1 }]);
        // (The ticks' inputs are the players' alone.)
        assert_eq!(b.session().remote_queue_length(0), 0);
    }
}
