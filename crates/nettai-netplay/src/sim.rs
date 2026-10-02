//! Two peers over simulated links, in one process: the rollback
//! simulator the tests drive. Each peer is a getgud [`Session`] on a
//! [`BattleWorld`] for its player's side.
//!
//! Each wall-clock frame, every peer takes the packets that have arrived
//! (a remote input with the sender's tick advantage) and reads its clock
//! skew (getgud's `skew`). A peer that runs ahead of the other stalls a
//! frame now and then (see [`SKEW_PER_STALL`]), and one whose unconfirmed
//! inputs reach `max_lead` waits for remote input (the stall guard). Every
//! other peer decides its player's input for its next tick and sends it;
//! then, having taken what arrived meanwhile, advances. With no latency a
//! packet arrived in the frame it was sent, so both peers confirm every
//! tick at once. A peer whose player's input has ended (at `max_frames`)
//! drains its session instead: it settles the last ticks as the other's
//! last inputs arrive, without speculating past them.
//!
//! Checked on every advance: each row the peer confirmed (getgud's
//! `Advance::confirmed`) is the inputs both players decided for its tick,
//! and every settled state getgud returns with a row has the digest of the
//! other peer's at that tick and of a plain lockstep run's.

use getgud::{Confirmed, Session, Settlement, World};

use crate::network::{Link, LinkConfig};
use crate::world::{BattleState, BattleWorld, Game, Observer, step_game};
use nettai_battle::{Battle, RoundEnd};

/// Clock sync: how much skew a peer that runs ahead adds up before it
/// stalls a frame. It slows down by `skew / SKEW_PER_STALL` frames a
/// frame, as a frame-rate adjustment would (a frame a second per tick of
/// skew, at 60 fps). A peer one frame ahead of the other shows a skew of
/// 2 (it leads by one more, the other by one less). Stalling on any
/// positive skew at once over-corrects: a stall shows in the skew in full
/// only a round trip later, and jitter makes the skew noisy, so the peers
/// would take turns stalling.
pub const SKEW_PER_STALL: i32 = 60;

/// How the peers are connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetConfig {
    /// Both directions.
    pub link: LinkConfig,
    /// Ticks each peer presents behind its newest local input (getgud's
    /// present delay): input delay, which spares that much prediction.
    pub present_delay: u32,
    /// The stall guard: a peer with this many local inputs that no remote
    /// input matches yet waits, unless remote input it has can be matched
    /// (getgud's `matchable`).
    pub max_lead: u32,
    /// Wall frames peer 0 starts before peer 1, for clock sync to even out.
    pub head_start: u32,
    /// Seeds the links' jitter.
    pub seed: u64,
}

impl NetConfig {
    /// `latency` frames each way, up to `jitter` more, no present delay.
    pub fn latency(latency: u32, jitter: u32) -> NetConfig {
        NetConfig {
            link: LinkConfig { latency, jitter },
            present_delay: 0,
            max_lead: (latency + jitter + 2).max(8),
            head_start: 0,
            seed: 0x5EED ^ (latency as u64) << 8 ^ jitter as u64,
        }
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

/// What a peer did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerStats {
    /// Ticks simulated for the first time.
    pub simulated: u64,
    /// Rollbacks: advances that found a misprediction and threw the
    /// speculative ticks from it away (getgud's `last_misprediction_depth`,
    /// when it isn't 0).
    pub rollbacks: u64,
    /// The ticks those rollbacks threw away, which are simulated again.
    pub resimulated: u64,
    /// The deepest rollback, in ticks.
    pub max_rollback: u32,
    /// The furthest a presented frame ran past the confirmed input, in
    /// ticks (getgud's `speculation_balance`).
    pub max_speculation: u32,
    /// The same, added up over every advance.
    pub speculated: u64,
    /// Advances (frames presented).
    pub advances: u64,
    /// Rows settled by promoting their speculation (the rest were simulated
    /// as they settled).
    pub promoted: u64,
    /// Wall frames stalled for clock sync: the peer ran ahead.
    pub stalls: u64,
    /// Wall frames the stall guard held the peer.
    pub parked: u64,
}

impl PeerStats {
    /// How far a presented frame ran past the confirmed input, on average.
    pub fn mean_speculation(&self) -> f64 {
        self.speculated as f64 / self.advances.max(1) as f64
    }
}

/// How a match went.
#[derive(Clone, Debug)]
pub struct Report {
    /// Ticks both peers settled.
    pub frames: u32,
    pub wall_frames: u64,
    pub peers: [PeerStats; 2],
    /// Both peers' settled states are over.
    pub over: bool,
    /// How the battle ended in both peers' settled states, if it did and
    /// they agree.
    pub end: Option<RoundEnd>,
    pub divergence: Option<Divergence>,
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
        self.divergence.is_none()
    }
}

/// Two peers from their starting states, and the lockstep reference.
pub struct Match<G: Game> {
    pub config: NetConfig,
    peers: [G; 2],
    lockstep: G,
}

impl<G: Game + Clone> Match<G> {
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
    /// `max_frames` ticks, or the peers fall out of sync. `inputs(player,
    /// tick)` decides a player's input; it is called once per tick and
    /// player, in tick order, for the ticks before `max_frames`, where the
    /// players' input ends: a peer drains its session from there, and no
    /// peer simulates a tick past it. `observers[p]` sees what happens to
    /// peer `p`'s simulation.
    pub fn run<O: Observer<G>>(
        self,
        mut inputs: impl FnMut(usize, u32) -> G::Input,
        observers: &mut [O; 2],
        max_frames: u32,
    ) -> Report {
        let Match { config, peers: [a, b], lockstep } = self;
        let [first, second] = observers;
        let mut sessions = [
            BattleWorld::with_observer(a, 0, Tally::new(first)).session(config.present_delay),
            BattleWorld::with_observer(b, 1, Tally::new(second)).session(config.present_delay),
        ];
        let link = |p: u64| Link::new(config.link, config.seed.wrapping_mul(0x9E37_79B9).wrapping_add(p));
        // links[p]: from peer p to the other; an input and the sender's
        // tick advantage.
        let mut links: [Link<(G::Input, i16)>; 2] = [link(0), link(1)];
        let mut referee = Referee::new(lockstep);
        let mut stats = [PeerStats::default(), PeerStats::default()];
        // Clock sync: each peer's skew added up, and whether it stalled
        // the last frame.
        let mut drift = [0i32; 2];
        let mut stalled = [false; 2];
        let settled = |s: &Session<_>| -> (u32, Option<RoundEnd>) {
            let state: &BattleState = s.settled_state();
            (state.tick(), state.battle().round_end().cloned())
        };
        let wall_limit = max_frames as u64 * 4 + 1000;
        let mut now = 0;
        'wall: while now < wall_limit {
            let ends = sessions.each_ref().map(settled);
            if ends.iter().all(|e| e.1.is_some()) || ends.iter().all(|e| e.0 >= max_frames) {
                break;
            }
            let started = [true, now >= config.head_start as u64];
            // What has arrived by now, before anyone sends this frame.
            for p in (0..2).filter(|&p| started[p]) {
                for (remote, advantage) in links[1 - p].receive(now) {
                    sessions[p].add_remote_input(0, remote, advantage);
                }
            }
            let mut deciding: [Option<G::Input>; 2] = [None, None];
            for p in (0..2).filter(|&p| started[p]) {
                let s = &mut sessions[p];
                if s.local_frontier() >= max_frames {
                    // The input has ended: drain below.
                    continue;
                }
                if s.local_queue_length() >= config.max_lead as usize && s.matchable() == 0 {
                    stats[p].parked += 1;
                    continue;
                }
                // Running ahead: add up the skew and stall a frame for every
                // SKEW_PER_STALL of it, but never two in a row, so that the
                // peer keeps sending its advantage and the two can't wait
                // on each other. Only while the presented frame speculates:
                // until then the present delay absorbs the lead.
                if s.speculation_balance() >= 0 {
                    drift[p] = (drift[p] + s.skew()).max(0);
                }
                if drift[p] >= SKEW_PER_STALL && !stalled[p] {
                    drift[p] -= SKEW_PER_STALL;
                    stalled[p] = true;
                    stats[p].stalls += 1;
                    continue;
                }
                stalled[p] = false;
                let input = inputs(p, s.local_frontier());
                referee.decided[p].push(input.clone());
                links[p].send(now, (input.clone(), s.local_tick_advantage()));
                deciding[p] = Some(input);
            }
            for p in (0..2).filter(|&p| started[p]) {
                let s = &mut sessions[p];
                for (remote, advantage) in links[1 - p].receive(now) {
                    s.add_remote_input(0, remote, advantage);
                }
                let draining = s.local_frontier() >= max_frames;
                let rows = match deciding[p].take() {
                    Some(input) => {
                        let Ok(advanced) = s.advance(input);
                        advanced.confirmed
                    }
                    None if draining => {
                        let Ok(rows) = s.drain();
                        rows
                    }
                    None => continue,
                };
                for row in &rows {
                    referee.confirmed(p, row);
                    stats[p].promoted += (row.settlement == Settlement::Promoted) as u64;
                }
                drop(rows);
                let depth = s.last_misprediction_depth();
                if depth > 0 {
                    stats[p].rollbacks += 1;
                    stats[p].resimulated += depth as u64;
                    stats[p].max_rollback = stats[p].max_rollback.max(depth);
                }
                if !draining {
                    let speculation = s.speculation_balance().max(0) as u32;
                    stats[p].max_speculation = stats[p].max_speculation.max(speculation);
                    stats[p].speculated += speculation as u64;
                    stats[p].advances += 1;
                }
                if referee.divergence.is_some() {
                    break 'wall;
                }
            }
            now += 1;
        }
        for (s, session) in stats.iter_mut().zip(&sessions) {
            s.simulated = session.world().observer().simulated;
        }
        let ends = sessions.each_ref().map(settled);
        let [(_, a), (_, b)] = &ends;
        Report {
            frames: ends[0].0.min(ends[1].0),
            wall_frames: now,
            peers: stats,
            over: ends.iter().all(|e| e.1.is_some()),
            end: a.clone().filter(|_| a == b),
            divergence: referee.divergence,
            settled: referee.settled,
            lockstep: referee.lockstep_digests,
        }
    }
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

impl<G, O: Observer<G>> Observer<G> for Tally<'_, O> {
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
    fn confirmed(&mut self, frame: u32, settled: Option<&Battle>) {
        self.inner.confirmed(frame, settled);
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
