//! Two peers over a simulated datagram network, in one process: the
//! rollback simulator the tests drive. Each peer is a [`Peer`]: a getgud
//! session on a [`BattleWorld`] for its player's side, and its end of the
//! rennet link. The network ([`Network`], one per direction) delays,
//! reorders, loses and duplicates the peers' datagrams; rennet delivers
//! each player's inputs to the other once, in order.
//!
//! Each wall-clock frame, every peer takes the datagrams that have arrived
//! and decides whether to wait (clock sync's stall, or the stall guard:
//! [`Peer::wait`]). Every other peer decides its player's input for its
//! next tick; then every peer sends its datagram (input or not: the acks
//! and the redundancy window go every frame); then every peer that decided,
//! having taken what arrived meanwhile, advances. With no latency a
//! datagram arrives in the frame it was sent, so both peers confirm every
//! tick at once. A peer whose player's input has ended (at `max_frames`)
//! drains its session instead ([`Peer::drain`]): it settles the last ticks
//! as the other's last inputs arrive, without speculating past them.
//!
//! Checked on every advance: each row the peer confirmed (getgud's
//! `Advance::confirmed`) is the inputs both players decided for its tick,
//! and every settled state getgud returns with a row has the digest of the
//! other peer's at that tick and of a plain lockstep run's. A link that
//! breaks (a gap past the horizon) tears the match down: the report says
//! where.

use getgud::{Confirmed, Session, Settlement, World};
use nettai_battle::{Battle, RoundEnd};

use crate::link::LinkStats;
use crate::network::{Network, NetworkConfig, NetworkStats};
use crate::peer::{Peer, PeerConfig};
pub use crate::peer::{PeerStats, SKEW_PER_STALL};
use crate::protocol::{self, WireInput};
use crate::world::{BattleState, BattleWorld, Game, Observer, step_game};

/// How the peers are connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetConfig {
    /// The network from each peer to the other.
    pub networks: [NetworkConfig; 2],
    /// Ticks each peer presents behind its newest local input (getgud's
    /// present delay): input delay, which spares that much prediction.
    pub present_delay: u32,
    /// The stall guard: a peer with this many local inputs that no remote
    /// input matches yet waits, unless remote input it has can be matched
    /// (getgud's `matchable`).
    pub max_lead: u32,
    /// The links' rollback horizon ([`protocol::HORIZON`]).
    pub horizon: u32,
    /// Wall frames peer 0 starts before peer 1, for clock sync to even out.
    pub head_start: u32,
    /// Seeds the network.
    pub seed: u64,
}

impl NetConfig {
    /// `latency` frames each way, up to `jitter` more (which reorders
    /// datagrams), nothing lost, no present delay.
    pub fn latency(latency: u32, jitter: u32) -> NetConfig {
        NetConfig::over(NetworkConfig::latency(latency, jitter))
    }

    /// The same, losing `loss` datagrams in a thousand (once one is lost,
    /// the next with `burst` in a thousand) and duplicating `duplicate`.
    pub fn lossy(latency: u32, jitter: u32, loss: u32, burst: u32, duplicate: u32) -> NetConfig {
        NetConfig::over(NetworkConfig::lossy(latency, jitter, loss, burst, duplicate))
    }

    /// Both directions like `network`.
    pub fn over(network: NetworkConfig) -> NetConfig {
        let lossy = network.loss > 0 || network.outage.is_some();
        NetConfig {
            networks: [network; 2],
            present_delay: 0,
            // A lost datagram's input comes with the next one: a frame or
            // more later.
            max_lead: (network.max_delay() + 2 + if lossy { 4 } else { 0 }).max(8),
            horizon: protocol::HORIZON,
            head_start: 0,
            seed: 0x5EED
                ^ (network.latency as u64) << 8
                ^ network.jitter as u64
                ^ (network.loss as u64) << 16
                ^ (network.burst as u64) << 26
                ^ (network.duplicate as u64) << 36,
        }
    }

    fn peer(&self) -> PeerConfig {
        PeerConfig { present_delay: self.present_delay, max_lead: self.max_lead, horizon: self.horizon }
    }
}

/// Where a peer's settled state disagreed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divergence {
    /// The settled state after this many ticks.
    pub tick: u32,
    /// The two peers' digests of it (none from a peer that hasn't returned
    /// a settled state for that tick).
    pub peers: [Option<u64>; 2],
    /// The lockstep run's.
    pub lockstep: u64,
}

/// A peer's link broke, which ends the match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TornDown {
    /// The peer whose link broke.
    pub peer: usize,
    pub wall_frame: u64,
    /// Its settled ticks then.
    pub settled: u32,
    pub reason: String,
}

/// How a match went.
#[derive(Clone, Debug)]
pub struct Report {
    /// Ticks both peers settled.
    pub frames: u32,
    pub wall_frames: u64,
    pub peers: [PeerStats; 2],
    /// Each peer's link.
    pub links: [LinkStats; 2],
    /// The network from each peer to the other.
    pub networks: [NetworkStats; 2],
    /// Both peers' settled states are over.
    pub over: bool,
    /// How the battle ended in both peers' settled states, if it did and
    /// they agree.
    pub end: Option<RoundEnd>,
    pub divergence: Option<Divergence>,
    pub torn_down: Option<TornDown>,
    /// Each peer's settled states that getgud returned with a row:
    /// (tick, digest), in tick order.
    pub settled: [Vec<(u32, u64)>; 2],
    /// The lockstep run's digest after each tick (0: the start), as far as
    /// the peers settled.
    pub lockstep: Vec<u64>,
}

impl Report {
    /// Ran to the end (or to the frame limit) with the peers in sync.
    pub fn in_sync(&self) -> bool {
        self.divergence.is_none() && self.torn_down.is_none()
    }
}

/// Two peers from their starting states, and the lockstep reference.
pub struct Match<G: Game> {
    pub config: NetConfig,
    peers: [G; 2],
    lockstep: G,
}

impl<G: Game + Clone> Match<G>
where
    G::Input: WireInput,
{
    /// Both peers start from `start`; peer `p` controls player `p`.
    pub fn new(start: &G, config: NetConfig) -> Match<G> {
        Match::from_starts([start.clone(), start.clone()], start.clone(), config)
    }

    /// Each peer and the lockstep run start from their own state (equal,
    /// unless a test wants otherwise).
    pub fn from_starts(peers: [G; 2], lockstep: G, config: NetConfig) -> Match<G> {
        Match { config, peers, lockstep }
    }

    /// Play until both peers' settled states are over, both have settled
    /// `max_frames` ticks, the peers fall out of sync or a link breaks.
    /// `inputs(player, tick)` decides a player's input; it is called once
    /// per tick and player, in tick order, for the ticks before
    /// `max_frames`, where the players' input ends: a peer drains its
    /// session from there, and no peer simulates a tick past it.
    /// `observers[p]` sees what happens to peer `p`'s simulation.
    pub fn run<O: Observer<G>>(
        self,
        mut inputs: impl FnMut(usize, u32) -> G::Input,
        observers: &mut [O; 2],
        max_frames: u32,
    ) -> Report {
        let Match { config, peers: [a, b], lockstep } = self;
        let [first, second] = observers;
        let mut peers = [
            Peer::new(BattleWorld::with_observer(a, 0, Tally::new(first)), config.peer()),
            Peer::new(BattleWorld::with_observer(b, 1, Tally::new(second)), config.peer()),
        ];
        let network = |p: usize| Network::new(config.networks[p], config.seed.wrapping_mul(0x9E37_79B9).wrapping_add(p as u64));
        // networks[p]: from peer p to the other.
        let mut networks = [network(0), network(1)];
        let mut referee = Referee::new(lockstep);
        let mut promoted = [0u64; 2];
        let mut torn_down = None;
        // A peer's settled tick, and how its settled state ended.
        let settled = |s: &Session<_>| -> (u32, Option<RoundEnd>) {
            let state: &BattleState = s.settled_state();
            (state.tick(), state.battle().round_end().cloned())
        };
        let wall_limit = max_frames as u64 * 4 + 1000;
        let mut now = 0;
        'wall: while now < wall_limit {
            let ends = peers.each_ref().map(|p| settled(p.session()));
            if ends.iter().all(|e| e.1.is_some()) || ends.iter().all(|e| e.0 >= max_frames) {
                break;
            }
            let started = [true, now >= config.head_start as u64];
            for p in (0..2).filter(|&p| started[p]) {
                if let Some(t) = take(p, now, &mut peers, &mut networks) {
                    torn_down = Some(t);
                    break 'wall;
                }
            }
            let mut deciding = [false; 2];
            for p in (0..2).filter(|&p| started[p]) {
                // (A peer whose input has ended drains below.)
                if peers[p].session().local_frontier() < max_frames && peers[p].wait().is_none() {
                    let input = inputs(p, peers[p].session().local_frontier());
                    referee.decided[p].push(input.clone());
                    peers[p].decide(input);
                    deciding[p] = true;
                }
                networks[p].send(now, peers[p].datagram(now));
            }
            for p in 0..2 {
                let draining = started[p] && peers[p].session().local_frontier() >= max_frames;
                if !deciding[p] && !draining {
                    continue;
                }
                if let Some(t) = take(p, now, &mut peers, &mut networks) {
                    torn_down = Some(t);
                    break 'wall;
                }
                let (referee, promoted) = (&mut referee, &mut promoted[p]);
                let mut check = |rows: &[Confirmed<'_, BattleWorld<G, Tally<'_, O>>>]| {
                    for row in rows {
                        referee.confirmed(p, row);
                        *promoted += (row.settlement == Settlement::Promoted) as u64;
                    }
                };
                if deciding[p] {
                    peers[p].advance(|advanced| check(&advanced.confirmed));
                } else {
                    peers[p].drain(|rows| check(&rows));
                }
                if referee.divergence.is_some() {
                    break 'wall;
                }
            }
            now += 1;
        }
        let stats = [0, 1].map(|p| PeerStats {
            simulated: peers[p].session().world().observer().simulated,
            promoted: promoted[p],
            ..peers[p].stats().clone()
        });
        let ends = peers.each_ref().map(|p| settled(p.session()));
        let [(_, a), (_, b)] = &ends;
        Report {
            frames: ends[0].0.min(ends[1].0),
            wall_frames: now,
            peers: stats,
            links: peers.each_ref().map(|p| p.link_stats().clone()),
            networks: networks.each_ref().map(|n| *n.stats()),
            over: ends.iter().all(|e| e.1.is_some()),
            end: a.clone().filter(|_| a == b),
            divergence: referee.divergence,
            torn_down,
            settled: referee.settled,
            lockstep: referee.lockstep_digests,
        }
    }
}

/// Peer `p` takes the datagrams that have arrived by `now`; a link that
/// breaks tears the match down.
fn take<G: Game, O: Observer<G>>(p: usize, now: u64, peers: &mut [Peer<G, O>; 2], networks: &mut [Network; 2]) -> Option<TornDown>
where
    G::Input: WireInput,
{
    for datagram in networks[1 - p].receive(now) {
        if let Err(e) = peers[p].receive(&datagram, now) {
            let settled = peers[p].session().settled_state().tick();
            return Some(TornDown { peer: p, wall_frame: now, settled, reason: e.to_string() });
        }
    }
    None
}

/// Follows a peer's world for the simulator (what it simulated for the
/// first time) and passes everything on.
struct Tally<'o, O> {
    inner: &'o mut O,
    /// Frames before this one were simulated at least once.
    reached: u32,
    simulated: u64,
}

impl<'o, O> Tally<'o, O> {
    fn new(inner: &'o mut O) -> Tally<'o, O> {
        Tally { inner, reached: 0, simulated: 0 }
    }
}

impl<G: Game, O: Observer<G>> Observer<G> for Tally<'_, O> {
    fn rolled_back(&mut self, frame: u32) {
        self.inner.rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G) {
        if frame >= self.reached {
            self.reached = frame + 1;
            self.simulated += 1;
        }
        self.inner.simulated(frame, game);
    }
    fn confirmed(&mut self, frame: u32, inputs: [&G::Input; 2], settled: Option<&Battle>) {
        self.inner.confirmed(frame, inputs, settled);
    }
}

/// The inputs both players decided, the lockstep run of them, and what
/// the peers settled.
struct Referee<G: Game> {
    /// Each player's inputs, by tick.
    decided: [Vec<G::Input>; 2],
    lockstep: G,
    /// The lockstep run's digest after each tick (0: the start).
    lockstep_digests: Vec<u64>,
    /// Each peer's next row's tick.
    next_row: [u32; 2],
    settled: [Vec<(u32, u64)>; 2],
    divergence: Option<Divergence>,
}

impl<G: Game + Clone> Referee<G> {
    fn new(lockstep: G) -> Referee<G> {
        Referee {
            decided: [Vec::new(), Vec::new()],
            lockstep_digests: vec![lockstep.battle().digest()],
            lockstep,
            next_row: [0, 0],
            settled: [Vec::new(), Vec::new()],
            divergence: None,
        }
    }

    /// Peer `p` confirmed this row: its own player's input and the other
    /// player's, which must be what the players decided for its tick, and
    /// the state after it, if getgud kept one, which must be the lockstep
    /// run's and the other peer's.
    fn confirmed<W: World<Input = G::Input, State = BattleState>>(&mut self, p: usize, row: &Confirmed<'_, W>) {
        let t = row.tick as usize;
        assert_eq!(row.tick, self.next_row[p], "peer {p}'s rows follow on");
        self.next_row[p] += 1;
        let decided = |player: usize| &self.decided[player][t];
        assert!(
            row.local == *decided(p) && row.remotes.len() == 1 && row.remotes[0] == *decided(1 - p),
            "peer {p} confirmed other inputs for tick {t} than its players decided"
        );
        if let Some(state) = row.state {
            assert_eq!(state.tick(), row.tick + 1);
            self.check_settled(p, state.tick(), state.battle().digest());
        }
    }

    /// The lockstep run's digest after `tick` ticks (both players decided
    /// their inputs that far). It steps as a peer does.
    fn lockstep_digest(&mut self, tick: u32) -> u64 {
        while self.lockstep_digests.len() <= tick as usize {
            let t = self.lockstep_digests.len() - 1;
            step_game(&mut self.lockstep, [&self.decided[0][t], &self.decided[1][t]]);
            self.lockstep_digests.push(self.lockstep.battle().digest());
        }
        self.lockstep_digests[tick as usize]
    }

    /// Peer `p`'s settled state after `tick` ticks has this digest.
    fn check_settled(&mut self, p: usize, tick: u32, digest: u64) {
        self.settled[p].push((tick, digest));
        let lockstep = self.lockstep_digest(tick);
        let other = &self.settled[1 - p];
        let other = other.binary_search_by_key(&tick, |s| s.0).ok().map(|i| other[i].1);
        if self.divergence.is_none() && (digest != lockstep || other.is_some_and(|d| d != digest)) {
            let mut peers = [None; 2];
            peers[p] = Some(digest);
            peers[1 - p] = other;
            self.divergence = Some(Divergence { tick, peers, lockstep });
        }
    }
}
