//! Synthetic netbattles under rollback: two MegaMen with fixed folders
//! and seeded random button mashing (which drives their custom screens
//! too), played by two getgud sessions over a simulated datagram network
//! (latency, jitter that reorders, loss, duplication), their inputs
//! carried by rennet. At every advance, each peer's settled state must
//! equal the other's and a plain lockstep run's, the battle must end, and
//! each peer's sound must play every confirmed cue once.

use nettai_battle::cues::CueAction;
use nettai_battle::{Battle, SoundCue, TickInput};
use nettai_netplay::battle::{PlayerInput, CueFeed};
use nettai_netplay::network::NetworkConfig;
use nettai_netplay::sim::{Match, NetConfig, Report};
use nettai_netplay::standin::{Masher, StandInBattle, folder, netbattle};
use nettai_netplay::{Game, Observer};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use nettai_battle::content::testing;

/// The engine's hand-authored test content (made up, not BN6's data).
fn content() -> Arc<nettai_battle::Content> {
    testing::content()
}

/// Side 0's folder: GunDelSol chips, a navi chip (the eraser navi) and a
/// dimming (the invisibility dimming chip). Side 1's: GunDelSols only. The
/// codes are the chips' own (A and *).
fn folders(c: &nettai_battle::Content) -> [nettai_battle::custom::BattleFolder; 2] {
    use testing::{ERASER, SUN_GUN_1, SUN_GUN_2, SUN_GUN_3, VEIL};
    [
        folder(c, &[(SUN_GUN_3, 0), (ERASER, 0), (SUN_GUN_1, 0), (VEIL, 26), (SUN_GUN_3, 26)]),
        folder(c, &[(SUN_GUN_3, 0), (SUN_GUN_1, 0), (SUN_GUN_2, 0), (SUN_GUN_3, 26), (SUN_GUN_1, 26)]),
    ]
}

fn start(seed: u64) -> StandInBattle {
    let c = content();
    StandInBattle::new(Battle::new(netbattle(&c, nettai_battle::content::testing::LINK_BATTLE, 300, seed as u32 ^ 0x1234_5678, folders(&c)), c))
}

fn mashers(seed: u64) -> impl FnMut(usize, u32) -> u16 {
    mashers_with(seed, false)
}

/// The same, pressing B too (`buster`): the buster's and the charged
/// shot's scripts and their projectiles.
fn mashers_with(seed: u64, buster: bool) -> impl FnMut(usize, u32) -> u16 {
    let mut m = [Masher::new(seed), Masher::new(seed ^ 0xABCD)];
    for x in &mut m {
        x.buster = buster;
    }
    move |p, _| m[p].buttons()
}

fn play(seed: u64, config: NetConfig) -> (Report, [CueFeed; 2]) {
    let mut feeds = [CueFeed::new(0, 3), CueFeed::new(1, 3)];
    let report = Match::new(&start(seed), config).run(mashers_with(seed, true), &mut feeds, 30_000);
    (report, feeds)
}

/// Each peer's sound: every confirmed cue played once (plays minus
/// cancels, per cue), and something to show for the prediction.
fn check_cues(feed: &CueFeed) -> (usize, usize) {
    let mut balance: HashMap<SoundCue, i64> = HashMap::new();
    let (mut plays, mut cancels) = (0, 0);
    for &(_, action) in &feed.log {
        match action {
            CueAction::Play(c) => {
                *balance.entry(c).or_default() += 1;
                plays += 1;
            }
            CueAction::Cancel(c) => {
                *balance.entry(c).or_default() -= 1;
                cancels += 1;
            }
        }
    }
    // Each cue the tracker hasn't settled yet (played for a frame near or
    // past the last confirmed one) may still be canceled or confirmed.
    let mut confirmed: HashMap<SoundCue, i64> = HashMap::new();
    for &(_, c) in &feed.confirmed {
        *confirmed.entry(c).or_default() += 1;
    }
    let unsettled = feed.tracker.unconfirmed() as i64;
    let cues: std::collections::HashSet<SoundCue> = balance.keys().chain(confirmed.keys()).copied().collect();
    let diff: i64 = cues.iter().map(|c| (balance.get(c).copied().unwrap_or(0) - confirmed.get(c).copied().unwrap_or(0)).abs()).sum();
    assert!(diff <= unsettled, "viewer {}: played-minus-canceled cues differ from the confirmed cues by {diff} ({unsettled} unsettled)", feed.viewer);
    (plays, cancels)
}

fn run_latency(latency: u32, jitter: u32, present_delay: u32) {
    let config = NetConfig { present_delay, ..NetConfig::latency(latency, jitter) };
    run(&format!("latency {latency}+{jitter} present delay {present_delay}"), config, latency > present_delay);
}

/// The three seeds' battles under `config`, in sync and to the end, with
/// rollbacks or without, and the peers' sound right.
fn run(what: &str, config: NetConfig, rollbacks: bool) {
    for seed in [1u64, 2, 3] {
        let (report, feeds) = play(seed, config);
        let [a, b] = &report.peers;
        let cues: Vec<(usize, usize)> = feeds.iter().map(check_cues).collect();
        eprintln!(
            "{what} seed {seed}: {} frames in {} wall frames, over {}; \
             rollbacks {}/{} (max depth {}/{}, resimulated {}/{}, speculated up to {}/{}); stalls {}/{}, parked {}/{}; cues played/canceled {:?}",
            report.frames,
            report.wall_frames,
            report.over,
            a.rollbacks,
            b.rollbacks,
            a.max_rollback,
            b.max_rollback,
            a.resimulated,
            b.resimulated,
            a.max_speculation,
            b.max_speculation,
            a.stalls,
            b.stalls,
            a.parked,
            b.parked,
            cues,
        );
        eprintln!("  {}", links(&report));
        assert!(report.in_sync(), "seed {seed}: {:?} {:?}", report.divergence, report.torn_down);
        assert!(matches!(report.end, Some(nettai_battle::RoundEnd::Over(_))), "seed {seed}: {:?}", report.end);
        assert!(report.over, "seed {seed}: the battle didn't end in {} frames", report.frames);
        if rollbacks {
            assert!(a.rollbacks > 0 && b.rollbacks > 0, "the latency should cause rollbacks");
        } else {
            assert_eq!((a.rollbacks, b.rollbacks), (0, 0), "no rollback without latency");
        }
    }
}

/// The links and the network, for the log.
fn links(report: &Report) -> String {
    let [a, b] = &report.links;
    let [x, y] = &report.networks;
    format!(
        "datagrams {}/{} of {:.1}/{:.1} bytes (largest {}/{}), elements {:.1}/{:.1} a datagram; \
         network lost {}/{} (longest burst {}/{}), duplicated {}/{}, reordered {}/{}; \
         redundant elements received {}/{}; loss {:.1}%/{:.1}%, estimated by the receiver {:.1}%/{:.1}%",
        a.sent,
        b.sent,
        a.mean_size(),
        b.mean_size(),
        a.largest,
        b.largest,
        a.sent_elements as f64 / a.sent.max(1) as f64,
        b.sent_elements as f64 / b.sent.max(1) as f64,
        x.lost,
        y.lost,
        x.longest_burst,
        y.longest_burst,
        x.duplicated,
        y.duplicated,
        x.reordered,
        y.reordered,
        a.redundant,
        b.redundant,
        x.lost as f64 * 100.0 / x.sent.max(1) as f64,
        y.lost as f64 * 100.0 / y.sent.max(1) as f64,
        b.loss() * 100.0,
        a.loss() * 100.0,
    )
}

#[test]
fn lockstep_no_latency() {
    run_latency(0, 0, 0);
}

#[test]
fn latency_1() {
    run_latency(1, 1, 0);
}

#[test]
fn latency_2() {
    run_latency(2, 1, 0);
}

#[test]
fn latency_5() {
    run_latency(5, 2, 0);
}

#[test]
fn latency_10() {
    run_latency(10, 3, 0);
}

#[test]
fn latency_10_with_present_delay() {
    run_latency(10, 2, 3);
}

// A lossy network: datagrams lost (alone or in bursts: once one is lost,
// the next often is too), duplicated, and reordered by jitter. rennet's
// redundancy window brings a lost datagram's inputs with the next one.

#[test]
fn loss_without_latency() {
    run("loss 10% (bursts 30%), duplicates 5%, latency 0", NetConfig::lossy(0, 0, 100, 300, 50), true);
}

#[test]
fn loss_and_reordering_at_latency_2() {
    run("loss 10% (bursts 30%), duplicates 5%, latency 2+2", NetConfig::lossy(2, 2, 100, 300, 50), true);
}

#[test]
fn loss_and_reordering_at_latency_5() {
    run("loss 15% (bursts 40%), duplicates 5%, latency 5+3", NetConfig::lossy(5, 3, 150, 400, 50), true);
}

#[test]
fn heavy_loss_at_latency_10() {
    run("loss 25% (bursts 50%), duplicates 10%, latency 10+4", NetConfig::lossy(10, 4, 250, 500, 100), true);
}

/// Nothing gets through either way for two seconds: both peers' stall
/// guards hold them, and the match goes on after it.
#[test]
fn an_outage_holds_both_peers_and_the_match_goes_on() {
    let network = NetworkConfig { outage: Some((1000, 1120)), ..NetworkConfig::latency(3, 1) };
    let (report, feeds) = play(1, NetConfig::over(network));
    feeds.iter().for_each(|f| {
        check_cues(f);
    });
    let [a, b] = &report.peers;
    eprintln!("outage: {} frames, parked {}/{}, deepest rollback {}/{}; {}", report.frames, a.parked, b.parked, a.max_rollback, b.max_rollback, links(&report));
    assert!(report.in_sync() && report.over, "{:?} {:?} {:?}", report.divergence, report.end, report.torn_down);
    assert!(a.parked >= 100 && b.parked >= 100);
}

/// A gap in a player's stream past the horizon can't be recovered in time:
/// the peer that sees it tears the match down, with the settled states in
/// agreement up to there. Two peers that keep a stall guard that fits the
/// horizon never open one (a gap reaches at most twice the stall guard),
/// so this takes a horizon too small for it: one direction fails for five
/// seconds, the peer that hears nothing waits at its stall guard, the
/// other runs on to its own, and when the link comes back the gap is
/// wider than the horizon. With the full horizon, the same match goes on.
#[test]
fn a_gap_past_the_horizon_tears_the_match_down() {
    let mut config = NetConfig::latency(2, 0);
    config.max_lead = 40;
    config.networks[0].outage = Some((300, 600));
    let full = Match::new(&start(1), config).run(mashers(1), &mut [(), ()], 3000);
    assert!(full.in_sync() && full.frames == 3000, "{:?} {:?}", full.divergence, full.torn_down);
    config.horizon = 32;
    let report = Match::new(&start(1), config).run(mashers(1), &mut [(), ()], 3000);
    let torn = report.torn_down.clone().expect("the match goes on past the gap");
    eprintln!("torn down: {torn:?}; settled {} frames", report.frames);
    assert_eq!(torn.peer, 1);
    assert!(torn.reason.contains("horizon"), "{}", torn.reason);
    assert!(report.divergence.is_none());
    assert!((290..310).contains(&torn.settled), "{torn:?}");
}

/// A set's rounds, played by two peers over the simulated network as a
/// frontend does (`Peer`): each ends a round once its settled state is
/// over and starts the next from the shared setup; the other player's
/// inputs past their round's end are dropped, those of their next round
/// wait for it. Both peers' settled states agree in every round, and when
/// the set is over one player leaves and the other hears it.
#[test]
fn a_sets_rounds_follow_each_other_on_one_stream() {
    use nettai_battle::setup::{RoundSetup, effects};
    use nettai_battle::{BattleResult, RoundEnd, Stage};
    use nettai_netplay::network::Network;
    use nettai_netplay::{BattleWorld, Peer, PeerConfig};
    let c = content();
    let first = {
        let mut s = netbattle(&c, testing::LINK_BATTLE, 40, 0x51, folders(&c));
        s.settings.effects |= effects::SET;
        s.later_stages = [Stage { stage: s.settings.stage, background: s.settings.background }; 2];
        s
    };
    let world = |setup: RoundSetup, side: usize| BattleWorld::new(StandInBattle::new(Battle::new(setup, c.clone())), side);
    let config = PeerConfig::new(0, 12);
    let mut peers = [Peer::new(world(first.clone(), 0), config), Peer::new(world(first.clone(), 1), config)];
    let mut networks = [0, 1].map(|p| Network::new(NetworkConfig::lossy(4, 2, 100, 300, 50), 77 + p));
    let mut mash = mashers_with(9, true);
    // Each peer's settled digests, by round: (tick, digest).
    let mut settled: [Vec<Vec<(u32, u64)>>; 2] = [vec![Vec::new()], vec![Vec::new()]];
    let mut results: [Option<BattleResult>; 2] = [None, None];
    let mut now = 0u64;
    while results.iter().any(Option::is_none) {
        assert!(now < 60_000, "the set doesn't end: rounds {:?}", peers.each_ref().map(|p| p.round()));
        for p in 0..2 {
            for d in networks[1 - p].receive(now) {
                peers[p].receive(&d, now).unwrap();
            }
        }
        let mut deciding = [false; 2];
        for p in 0..2 {
            if results[p].is_none() && peers[p].wait().is_none() {
                peers[p].decide(mash(p, 0));
                deciding[p] = true;
            }
            networks[p].send(now, peers[p].datagram(now));
        }
        for p in (0..2).filter(|&p| deciding[p]) {
            peers[p].advance(|_| ());
            let state = peers[p].session().settled_state();
            settled[p].last_mut().unwrap().push((state.tick(), state.battle().digest()));
            match state.battle().round_end() {
                None => {}
                Some(RoundEnd::NextRound { settings, score }) => {
                    let next = RoundSetup { settings: *settings, score: *score, ..first.clone() };
                    peers[p].end_round(world(next, p));
                    settled[p].push(Vec::new());
                }
                Some(RoundEnd::Over(result)) => results[p] = Some(*result),
                Some(RoundEnd::Error(message)) => panic!("peer {p}'s round stopped: {message}"),
            }
        }
        now += 1;
    }
    let rounds = settled.each_ref().map(|r| r.len());
    eprintln!(
        "a set of {} rounds in {now} wall frames: {:?}; rollbacks {}/{}; {:?}",
        rounds[0],
        results,
        peers[0].stats().rollbacks,
        peers[1].stats().rollbacks,
        settled[0].iter().map(|r| r.last().map_or(0, |s| s.0)).collect::<Vec<_>>(),
    );
    assert_eq!(rounds[0], rounds[1]);
    assert!(rounds[0] >= 2, "one round only");
    for (r, (a, b)) in settled[0].iter().zip(&settled[1]).enumerate() {
        let b: std::collections::HashMap<u32, u64> = b.iter().copied().collect();
        let common = a.iter().filter(|(t, d)| b.get(t).is_some_and(|e| {
            assert_eq!(e, d, "round {} tick {t}: the peers' settled states differ", r + 1);
            true
        }));
        assert!(common.count() > 100, "round {}", r + 1);
    }
    // (The result is the shared simulation's, side 0's.)
    assert_eq!(results[0], results[1]);
    peers[0].leave();
    for t in now..now + 30 {
        networks[0].send(t, peers[0].datagram(t));
        for d in networks[0].receive(t) {
            peers[1].receive(&d, t).unwrap();
        }
    }
    assert!(peers[1].remote_left());
}

/// The engine's own input record, with the frame's events riding in
/// player 0's input: record a synthetic battle's tick inputs, then play
/// them back through `Battle` as a rollback game.
#[test]
fn recorded_events_ride_in_the_inputs() {
    let mut g = start(7);
    let mut inputs = mashers(7);
    let mut record: Vec<TickInput> = Vec::new();
    while g.battle.round_end().is_none() {
        let buttons = [inputs(0, 0), inputs(1, 0)];
        let t = g.tick_input(buttons);
        g.battle.step(&t);
        record.push(t);
    }
    let shares = |p: usize, f: u32| {
        let t = &record[(f as usize).min(record.len() - 1)];
        PlayerInput { tick: t.players[p], events: if p == 0 { t.events.clone() } else { Default::default() } }
    };
    let final_digest = g.battle.digest();
    for latency in [3, 8] {
        let report = Match::new(&start(7).battle, NetConfig::latency(latency, 2)).run(shares, &mut [(), ()], 30_000);
        assert!(report.in_sync() && report.over, "latency {latency}: {:?} {:?}", report.divergence, report.end);
        assert!(report.frames as usize >= record.len());
        assert_eq!(report.lockstep[record.len()], final_digest);
        for settled in &report.settled {
            // Settled at the end of the record or later, and in agreement
            // with the lockstep run all the way (in_sync).
            assert!(settled.last().is_some_and(|&(tick, _)| tick as usize >= record.len()));
        }
        eprintln!("recorded events, latency {latency}: {} frames, {} rollbacks", report.frames, report.peers[1].rollbacks);
    }
}

/// `local_side` is shared setup, not the viewer: two peers that simulate
/// from different sides disagree as soon as the intro starts (who fades
/// in), and parts of that stay in the state long after.
#[test]
fn the_local_side_is_part_of_the_shared_setup() {
    let mut a = start(5);
    let mut b = start(5);
    let c = content();
    b.battle = Battle::new(nettai_battle::RoundSetup { local_side: 1, ..netbattle(&c, nettai_battle::content::testing::LINK_BATTLE, 300, 5 ^ 0x1234_5678, folders(&c)) }, c);
    let mut inputs = mashers(5);
    let first_difference = (0..200u32).find(|_| {
        let i = [inputs(0, 0), inputs(1, 0)];
        a.step([&i[0], &i[1]]);
        b.step([&i[0], &i[1]]);
        a.battle.digest() != b.battle.digest()
    });
    assert!(first_difference.is_some_and(|f| f < 5), "{first_difference:?}");
}

/// Observers see every simulated frame once more per rollback, and the
/// settled frames in order, each once, with the state after it where getgud
/// kept one (the same states the simulator checks). The players' input
/// ends at 3,000 frames: the peers settle all of them, and no peer
/// simulates a frame past them.
#[test]
fn observers_see_every_simulated_and_settled_frame() {
    struct Count {
        settled: u32,
        with_state: Vec<u32>,
        simulated: u64,
        resimulated: u64,
        reached: u32,
    }
    impl<G: Game> Observer<G> for Count {
        fn simulated(&mut self, frame: u32, _: &G) {
            if frame < self.reached {
                self.resimulated += 1;
            } else {
                assert_eq!(frame, self.reached, "frames are first simulated in order");
                self.reached += 1;
                self.simulated += 1;
            }
        }
        fn confirmed(&mut self, frame: u32, settled: Option<&Battle>) {
            assert_eq!(frame, self.settled, "frames settle in order, each once");
            assert!(frame < self.reached);
            self.settled += 1;
            if let Some(settled) = settled {
                let _ = settled.digest();
                self.with_state.push(frame + 1);
            }
        }
    }
    let new = || Count { settled: 0, with_state: Vec::new(), simulated: 0, resimulated: 0, reached: 0 };
    let mut counts = [new(), new()];
    let config = NetConfig { present_delay: 1, ..NetConfig::latency(4, 4) };
    let report = Match::new(&start(4), config).run(mashers(4), &mut counts, 3000);
    assert!(report.in_sync());
    assert_eq!(report.frames, 3000);
    for ((c, stats), settled) in counts.iter().zip(&report.peers).zip(&report.settled) {
        assert_eq!(c.settled, 3000);
        assert_eq!(c.reached, 3000, "nothing simulated past the end of the input");
        assert_eq!(c.with_state, settled.iter().map(|s| s.0).collect::<Vec<_>>());
        assert_eq!((c.simulated, c.resimulated), (stats.simulated, stats.resimulated));
        assert!(stats.rollbacks > 0);
        // Most rows' predictions held, and their states came along.
        assert!(stats.promoted > 1500 && c.with_state.len() as u64 >= stats.promoted, "{stats:?}");
    }
}

/// An input the players never send (the mashers press neither SELECT nor
/// START; it goes on the wire like any buttons): a tick that gets it panics.
const POISON: u16 = nettai_battle::input::keys::SELECT | nettai_battle::input::keys::START;

/// The stand-in battle, but a tick on the poison input panics (as a
/// content error or a state the original can't go on from would), and
/// what it predicts after a player held L is the poison.
#[derive(Clone)]
struct Brittle(StandInBattle);

impl Game for Brittle {
    type Input = u16;
    fn step(&mut self, inputs: [&u16; 2]) {
        let clean = inputs.map(|i| i & !POISON);
        self.0.step([&clean[0], &clean[1]]);
        assert!(inputs.iter().all(|&&i| i & POISON != POISON), "the poison input");
    }
    fn battle(&self) -> &Battle {
        &self.0.battle
    }
    fn battle_mut(&mut self) -> &mut Battle {
        &mut self.0.battle
    }
    fn predict(last: &u16) -> u16 {
        if last & nettai_battle::input::keys::L != 0 { last | POISON } else { *last }
    }
}

/// Keep the panics of ticks that stop a battle (caught: they are what
/// these tests are about) off stderr; any other panic, a failing assertion
/// for one, prints as usual.
fn quiet_stopped_ticks() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = info.payload().downcast_ref::<&str>().copied().or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()));
            let stopped = message.is_some_and(|m| m.contains("the poison input") || m.contains("is not filled"));
            if !stopped {
                default(info);
            }
        }));
    });
}

/// Counts the simulated frames on which the battle had stopped.
#[derive(Default)]
struct Stops(u64);

impl Observer<Brittle> for Stops {
    fn simulated(&mut self, _: u32, game: &Brittle) {
        self.0 += game.0.battle.is_stopped() as u64;
    }
}

/// A tick that panics on predicted input only (the poison a peer predicts
/// after L) stops the speculated battle and nothing else: the real input
/// rolls it back, and the match runs to the KO in sync, as without the
/// poison.
#[test]
fn a_tick_that_panics_on_predicted_input_is_rolled_back() {
    quiet_stopped_ticks();
    let brittle = Brittle(start(2));
    let mut stops = [Stops::default(), Stops::default()];
    let report = Match::new(&brittle, NetConfig::latency(5, 2)).run(mashers(2), &mut stops, 30_000);
    let plain = Match::new(&start(2), NetConfig::latency(5, 2)).run(mashers(2), &mut [(), ()], 30_000);
    eprintln!("predicted poison: {} and {} speculated ticks stopped; {} frames", stops[0].0, stops[1].0, report.frames);
    assert!(report.in_sync(), "{:?}", report.divergence);
    assert!(stops.iter().all(|s| s.0 > 0), "speculation reached the poison");
    assert_eq!(report.end, plain.end);
    assert!(matches!(report.end, Some(nettai_battle::RoundEnd::Over(_))));
    assert_eq!(report.frames, plain.frames);
}

/// A tick that panics on confirmed input (player 1 sends the poison at
/// frame 900) stops the battle on both peers alike, the lockstep run too:
/// the match ends in sync, with the panic's message.
#[test]
fn a_tick_that_panics_on_confirmed_input_ends_the_match_on_both_peers() {
    quiet_stopped_ticks();
    let mut mash = mashers(3);
    let inputs = move |p: usize, f: u32| {
        let b = mash(p, f) & !nettai_battle::input::keys::L;
        if p == 1 && f == 900 { b | POISON } else { b }
    };
    let report = Match::new(&Brittle(start(3)), NetConfig::latency(4, 2)).run(inputs, &mut [(), ()], 30_000);
    assert!(report.in_sync(), "{:?}", report.divergence);
    assert!(report.over);
    assert_eq!(report.end, Some(nettai_battle::RoundEnd::Error("the poison input".into())));
    // Stopped at frame 900: the states after it are all the same.
    let stopped = report.lockstep[901];
    assert!(report.lockstep[901..].iter().all(|&d| d == stopped));
    assert_ne!(report.lockstep[900], stopped);
}

/// Clock sync: a peer that starts ahead runs ahead of the other's input
/// and speculates deeper, its skew says so, and it stalls frames until the
/// two are even; then the match goes on in sync, both peers speculating
/// as deep as from an even start.
#[test]
fn a_peer_that_runs_ahead_stalls() {
    let even = Match::new(&start(6), NetConfig::latency(3, 1)).run(mashers(6), &mut [(), ()], 3000);
    assert!(even.in_sync());
    let level = even.peers[0].mean_speculation();
    for head_start in [6, 20] {
        let config = NetConfig { head_start, ..NetConfig::latency(3, 1) };
        let ahead = Match::new(&start(6), config).run(mashers(6), &mut [(), ()], 3000);
        assert!(ahead.in_sync());
        let [a, b] = &ahead.peers;
        eprintln!(
            "head start {head_start}: stalls {}/{} (even start {}/{}), parked {}/{}, mean speculation {:.2}/{:.2} (even start {:.2}/{:.2})",
            a.stalls,
            b.stalls,
            even.peers[0].stalls,
            even.peers[1].stalls,
            a.parked,
            b.parked,
            a.mean_speculation(),
            b.mean_speculation(),
            level,
            even.peers[1].mean_speculation(),
        );
        // Peer 0 gave its head start back (waiting at the stall guard
        // counts too) ...
        assert!(a.stalls > even.peers[0].stalls && a.stalls + a.parked >= head_start as u64);
        // ... and the two ran even: neither speculates deeper than from an
        // even start, give or take the time it took.
        for p in [a, b] {
            assert!((p.mean_speculation() - level).abs() < 0.5, "{p:?}");
        }
    }
}

/// A game that keeps part of its state outside what a snapshot copies
/// (here a counter shared by the game and its snapshots, the way an `Rc`
/// or a static would be): a rollback can't restore it, the re-simulation
/// differs, and the digest check catches the peers falling out of sync.
#[test]
fn state_outside_the_snapshot_is_caught() {
    #[derive(Clone)]
    struct Leaky {
        game: StandInBattle,
        /// A presses so far, outside the snapshot.
        presses: Rc<Cell<u32>>,
    }
    impl Game for Leaky {
        type Input = u16;
        fn step(&mut self, inputs: [&u16; 2]) {
            // Only every fourth A press gets through.
            let a = inputs[0] & nettai_battle::input::keys::A != 0;
            self.presses.set(self.presses.get() + a as u32);
            let held = if a && !self.presses.get().is_multiple_of(4) { inputs[0] & !nettai_battle::input::keys::A } else { *inputs[0] };
            self.game.step([&held, inputs[1]]);
        }
        fn battle(&self) -> &Battle {
            &self.game.battle
        }
        fn battle_mut(&mut self) -> &mut Battle {
            &mut self.game.battle
        }
    }
    let leaky = || Leaky { game: start(1), presses: Rc::new(Cell::new(0)) };
    let report = Match::from_starts([leaky(), leaky()], leaky(), NetConfig::latency(4, 2)).run(mashers(1), &mut [(), ()], 30_000);
    let d = report.divergence.expect("the leak goes unnoticed");
    eprintln!("state outside the snapshot: out of sync at tick {} ({} rollbacks by then)", d.tick, report.peers[0].rollbacks);
    // Without latency there is no rollback, and nothing to notice.
    let report = Match::from_starts([leaky(), leaky()], leaky(), NetConfig::latency(0, 0)).run(mashers(1), &mut [(), ()], 30_000);
    assert!(report.in_sync());
}

/// Counts the simulated frames on which the battle had stopped, and keeps
/// the first stop's message; and what the battles reached: frames with a
/// navi in a Cross, in a Beast form, dimmed.
#[derive(Default)]
struct Failures {
    stopped: u64,
    first: Option<String>,
    cross: u64,
    beast: u64,
    dimmed: u64,
}

impl Observer<StandInBattle> for Failures {
    fn simulated(&mut self, frame: u32, game: &StandInBattle) {
        use bn6_compat::forms::{Kind, kind};
        let b = &game.battle;
        if let Some(nettai_battle::RoundEnd::Error(message)) = b.round_end() {
            self.stopped += 1;
            self.first.get_or_insert_with(|| format!("frame {frame}: {message}"));
        }
        let kinds = [0, 1].map(|side| kind(&b.content, b.stats[side].form));
        self.cross += kinds.iter().any(|k| k.is_some_and(Kind::has_cross)) as u64;
        self.beast += kinds.iter().any(|k| k.is_some_and(Kind::is_beast)) as u64;
        self.dimmed += b.is_dimmed() as u64;
    }
}

/// Mashed battles with everything the test content has on both sides:
/// folders of dimming chips (so cut-ins and counter cut-ins), navi chips,
/// giga cut-in chips, bombs, swords, traps and grabs, Crosses and Beast Out
/// unlocked (the custom screen's buttons; the test content's MegaMan has no
/// Cross or Beast form, which the golden traces cover), and 500 HP. No tick
/// fails, settled or speculated: the engine has every path mashing reaches
/// (a speculated tick is a tick on inputs a player could have pressed).
/// (Slow in a debug build: `--ignored`.)
#[test]
#[ignore]
fn mashed_battles_with_everything_never_stop() {
    use bn6_compat::Unlocks;
    use nettai_battle::custom::GameVersion;
    use testing::*;
    let c = content();
    let codes = |keys: &[&str]| -> Vec<(String, u8)> {
        keys.iter()
            .map(|&k| {
                let h = c.defs.chip_by_key(k).unwrap_or_else(|| panic!("no chip {k}"));
                (k.to_string(), c.chip(h).codes.first().map_or(0, |code| code.0))
            })
            .collect()
    };
    let a = codes(&[VEIL, FALZAR, SUN_GUN_3, BOMB, BLADE, ANTI_NAVI, ERASER, TRAP, AREA_GRAB, ELEM_TRAP, TOMAHAWK, HEAT]);
    let b = codes(&[GREGAR, VEIL, STEP_BLADE, FLASH, SEED, TIME_BOMB, DRAGON, BEES, PANEL_GRAB, SPOUT, ELEC, SLASH]);
    fn as_refs(v: &[(String, u8)]) -> Vec<(&str, u8)> {
        v.iter().map(|(k, code)| (k.as_str(), *code)).collect()
    }
    quiet_stopped_ticks();
    for seed in [11u64, 12, 13] {
        let mut setup = netbattle(&c, LINK_BATTLE, 500, seed as u32, [folder(&c, &as_refs(&a)), folder(&c, &as_refs(&b))]);
        for (p, version) in setup.players.iter_mut().zip([GameVersion::Falzar, GameVersion::Gregar]) {
            Unlocks::everything(version).write(&c, p).expect("the unlocks");
        }
        let start = StandInBattle::new(Battle::new(setup, c.clone()));
        let mut failures = [Failures::default(), Failures::default()];
        let report = Match::new(&start, NetConfig::latency(5, 2)).run(mashers_with(seed, true), &mut failures, 40_000);
        let [f, _] = &failures;
        eprintln!(
            "everything, seed {seed}: {} frames, end {:?}; peer 0 simulated {} frames in a Cross, {} in a Beast form, {} dimmed; \
             stopped ticks {}/{} ({:?})",
            report.frames,
            report.end,
            f.cross,
            f.beast,
            f.dimmed,
            failures[0].stopped,
            failures[1].stopped,
            failures.iter().find_map(|f| f.first.clone())
        );
        assert!(report.in_sync(), "seed {seed}: {:?}", report.divergence);
        assert!(!matches!(report.end, Some(nettai_battle::RoundEnd::Error(_))), "seed {seed}: {:?}", report.end);
        assert!(failures.iter().all(|f| f.stopped == 0), "seed {seed}: {:?}", failures.iter().find_map(|f| f.first.clone()));
    }
}

/// Every battle above runs the test content's scripts (GunDelSol, the
/// eraser navi chip, the buster: docs/design/scripting.md), so these tests
/// are also the scripted content under rollback.
#[test]
fn the_battles_run_the_content_scripts() {
    let battle = start(1).battle;
    let runtime = nettai_battle::behavior::Behaviors::for_content(&battle.content).unwrap();
    assert_eq!(runtime.runtime(), "luau");
}
