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
//! tick at once.
//!
//! Checked on every advance: the rows the peer confirmed (getgud's
//! `Advance::confirmed`) are the inputs both players decided, and the
//! digest of its settled state equals the other peer's at that tick and a
//! plain lockstep run's. A link that breaks (a gap past the horizon) tears
//! the match down: the report says where.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use nettai_battle::Battle;
use getgud::Session;

use crate::link::LinkStats;
use crate::network::{Network, NetworkConfig, NetworkStats};
use crate::peer::{Peer, PeerConfig};
pub use crate::peer::{PeerStats, SKEW_PER_STALL};
use crate::protocol::{self, WireInput};
use crate::world::{BattleState, BattleWorld, Game, Observer};

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
    /// The two peers' digests of it (none from a peer that didn't settle
    /// at that tick).
    pub peers: [Option<u64>; 2],
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
    pub divergence: Option<Divergence>,
    pub panicked: Option<Panicked>,
    pub torn_down: Option<TornDown>,
    /// Each peer's settled state's digest after every advance that settled
    /// ticks: (tick, digest).
    pub settled: [Vec<(u32, u64)>; 2],
    /// The lockstep run's digest after each tick (0: the start), as far as
    /// the peers settled.
    pub lockstep: Vec<u64>,
}

impl Report {
    /// Ran to the end (or to the frame limit) with the peers in sync.
    pub fn in_sync(&self) -> bool {
        self.divergence.is_none() && self.panicked.is_none() && self.torn_down.is_none()
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
    /// `max_frames` ticks (no peer presents a tick past it), the peers fall
    /// out of sync or a link breaks. `inputs(player, tick)` decides a
    /// player's input; it is called once per tick and player, in tick order
    /// (past `max_frames` too). `observers[p]` sees what peer `p` simulates
    /// and settles.
    pub fn run<O: Observer<G>>(
        self,
        mut inputs: impl FnMut(usize, u32) -> G::Input,
        observers: &mut [O; 2],
        max_frames: u32,
    ) -> Report {
        let Match { config, peers: [a, b], lockstep } = self;
        let [first, second] = observers;
        let tallies = [RefCell::new(Tally::new(first)), RefCell::new(Tally::new(second))];
        let mut peers = [
            Peer::new(BattleWorld::with_observer(a, 0, &tallies[0]), config.peer()),
            Peer::new(BattleWorld::with_observer(b, 1, &tallies[1]), config.peer()),
        ];
        let network = |p: usize| Network::new(config.networks[p], config.seed.wrapping_mul(0x9E37_79B9).wrapping_add(p as u64));
        // networks[p]: from peer p to the other.
        let mut networks = [network(0), network(1)];
        let mut referee = Referee::new(lockstep);
        let mut panicked = None;
        let mut torn_down = None;
        // A peer's settled tick, and whether its settled state is over.
        let settled = |s: &Session<_>| -> (u32, bool) {
            let state: &BattleState = s.settled_state();
            (state.tick(), state.battle().round_end().is_some())
        };
        let wall_limit = max_frames as u64 * 4 + 1000;
        let mut now = 0;
        'wall: while now < wall_limit {
            let ends = peers.each_ref().map(|p| settled(p.session()));
            if ends.iter().all(|e| e.1) || ends.iter().all(|e| e.0 >= max_frames) {
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
                if peers[p].wait().is_none() {
                    let input = inputs(p, peers[p].session().local_frontier());
                    referee.decided[p].push(input.clone());
                    peers[p].decide(input);
                    deciding[p] = true;
                }
                networks[p].send(now, peers[p].datagram(now));
            }
            for p in (0..2).filter(|&p| deciding[p]) {
                if let Some(t) = take(p, now, &mut peers, &mut networks) {
                    torn_down = Some(t);
                    break 'wall;
                }
                let peer = &mut peers[p];
                // Present nothing past the end.
                let s = peer.session_mut();
                s.set_present_delay(config.present_delay.max(s.local_frontier().saturating_sub(max_frames)));
                let rows = match catch_unwind(AssertUnwindSafe(|| peer.advance(|advanced| advanced.confirmed))) {
                    Ok(rows) => rows,
                    Err(e) => {
                        let frame = tallies[p].borrow().parked;
                        let message = e
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_default();
                        let speculative = referee.lockstep_passes(frame);
                        panicked = Some(Panicked { peer: p, frame, speculative, message });
                        break 'wall;
                    }
                };
                if !rows.is_empty() {
                    referee.confirmed_rows(p, &rows);
                    let state = peer.session().settled_state();
                    assert_eq!(state.tick(), referee.confirmed[p], "the settled state is after every confirmed row");
                    referee.settled(p, state.tick(), state.battle().digest());
                    Observer::<G>::confirmed(&mut *tallies[p].borrow_mut(), state.tick(), state.battle());
                    if referee.divergence.is_some() {
                        break 'wall;
                    }
                }
            }
            now += 1;
        }
        let stats = [0, 1].map(|p| PeerStats { simulated: tallies[p].borrow().simulated, ..peers[p].stats().clone() });
        let ends = peers.each_ref().map(|p| settled(p.session()));
        Report {
            frames: ends[0].0.min(ends[1].0),
            wall_frames: now,
            peers: stats,
            links: peers.each_ref().map(|p| p.link_stats().clone()),
            networks: networks.each_ref().map(|n| *n.stats()),
            over: ends.iter().all(|e| e.1),
            divergence: referee.divergence,
            panicked,
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

/// Follows a peer's world for the simulator (where it is parked, what it
/// simulated for the first time) and passes everything on.
struct Tally<'o, O> {
    inner: &'o mut O,
    /// The next frame the world simulates.
    parked: u32,
    /// Frames before this one were simulated at least once.
    reached: u32,
    simulated: u64,
}

impl<'o, O> Tally<'o, O> {
    fn new(inner: &'o mut O) -> Tally<'o, O> {
        Tally { inner, parked: 0, reached: 0, simulated: 0 }
    }
}

impl<G, O: Observer<G>> Observer<G> for Tally<'_, O> {
    fn rolled_back(&mut self, frame: u32) {
        self.parked = frame;
        self.inner.rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G) {
        self.parked = frame + 1;
        if frame >= self.reached {
            self.reached = frame + 1;
            self.simulated += 1;
        }
        self.inner.simulated(frame, game);
    }
    fn confirmed(&mut self, frames: u32, settled: &Battle) {
        self.inner.confirmed(frames, settled);
    }
}

/// A confirmed input row as getgud returns it: the peer's player's input,
/// and the other player's (the one remote slot).
type Row<I> = (I, Box<[I]>);

/// The inputs both players decided, the lockstep run of them, and what
/// the peers settled.
struct Referee<G: Game> {
    /// Each player's inputs, by tick.
    decided: [Vec<G::Input>; 2],
    lockstep: G,
    /// The lockstep run's digest after each tick (0: the start).
    lockstep_digests: Vec<u64>,
    /// Rows each peer confirmed.
    confirmed: [u32; 2],
    settled: [Vec<(u32, u64)>; 2],
    divergence: Option<Divergence>,
}

impl<G: Game + Clone> Referee<G> {
    fn new(lockstep: G) -> Referee<G> {
        Referee {
            decided: [Vec::new(), Vec::new()],
            lockstep_digests: vec![lockstep.battle().digest()],
            lockstep,
            confirmed: [0, 0],
            settled: [Vec::new(), Vec::new()],
            divergence: None,
        }
    }

    /// Peer `p` confirmed these rows: its own player's input and the other
    /// player's, which must be what the players decided.
    fn confirmed_rows(&mut self, p: usize, rows: &[Row<G::Input>]) {
        for (local, remotes) in rows {
            let t = self.confirmed[p] as usize;
            let decided = |player: usize| &self.decided[player][t];
            assert!(
                local == decided(p) && remotes.len() == 1 && &remotes[0] == decided(1 - p),
                "peer {p} confirmed other inputs for tick {t} than its players decided"
            );
            self.confirmed[p] += 1;
        }
    }

    /// The lockstep run's digest after `tick` ticks (both players decided
    /// their inputs that far).
    fn lockstep_digest(&mut self, tick: u32) -> u64 {
        while self.lockstep_digests.len() <= tick as usize {
            let t = self.lockstep_digests.len() - 1;
            self.lockstep.step([&self.decided[0][t], &self.decided[1][t]]);
            self.lockstep_digests.push(self.lockstep.battle().digest());
        }
        self.lockstep_digests[tick as usize]
    }

    /// Peer `p`'s settled state after `tick` ticks has this digest.
    fn settled(&mut self, p: usize, tick: u32, digest: u64) {
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

    /// Whether the lockstep run gets through `frame` (catching a panic).
    fn lockstep_passes(&self, frame: u32) -> bool {
        let decided = self.decided[0].len().min(self.decided[1].len());
        if frame as usize >= decided {
            return false;
        }
        let mut g = self.lockstep.clone();
        let from = self.lockstep_digests.len() - 1;
        catch_unwind(AssertUnwindSafe(|| {
            for t in from..=frame as usize {
                g.step([&self.decided[0][t], &self.decided[1][t]]);
            }
        }))
        .is_ok()
    }
}
