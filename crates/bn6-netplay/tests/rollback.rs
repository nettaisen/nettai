//! Synthetic netbattles under rollback: two MegaMen with fixed hands and
//! seeded random button mashing, played by two peers over simulated links
//! with various latencies. Every confirmed frame, both peers' state
//! digests must equal each other and a plain lockstep run's, the battle
//! must end, and each peer's sound must play every confirmed cue once.

use bn6_battle::cues::CueAction;
use bn6_battle::{Battle, SoundCue, TickInput};
use bn6_netplay::bn6::{Bn6Input, CueFeed};
use bn6_netplay::network::LinkConfig;
use bn6_netplay::sim::{Match, NetConfig, Report};
use bn6_netplay::standin::{Masher, Rules, StandInBattle, hand, netbattle};
use bn6_netplay::{Game, Observer};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Side 0: GunDelSols, EraseMan (a navi chip) and Invisibl (a time
/// freeze). Side 1: GunDelSols only, so that neither side can counter a
/// freeze with one of its own (not implemented yet).
fn rules() -> Rules {
    Rules {
        hands: [
            hand(&[(0x11, 13), (0xEC, 4), (0x0F, 2), (0xB1, 0), (0x11, 16)]),
            hand(&[(0x11, 13), (0x0F, 2), (0x10, 1), (0x11, 16), (0x0F, 12)]),
        ],
        min_ticks: 20,
        max_ticks: 90,
        beast_out: false,
    }
}

fn start(seed: u64) -> StandInBattle {
    StandInBattle::new(Battle::new(netbattle(500, seed as u32 ^ 0x1234_5678)), rules())
}

fn mashers(seed: u64) -> impl FnMut(usize, u32) -> u16 {
    let mut m = [Masher::new(seed), Masher::new(seed ^ 0xABCD)];
    move |p, _| m[p].buttons()
}

fn play(seed: u64, config: NetConfig) -> (Report, [CueFeed; 2]) {
    let mut feeds = [CueFeed::new(0, 3), CueFeed::new(1, 3)];
    let report = Match::new(&start(seed), config).run(mashers(seed), &mut feeds, 30_000);
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
    // past the last confirmed one) may still be cancelled or confirmed.
    let mut confirmed: HashMap<SoundCue, i64> = HashMap::new();
    for &(_, c) in &feed.confirmed {
        *confirmed.entry(c).or_default() += 1;
    }
    let unsettled = feed.tracker.unconfirmed() as i64;
    let cues: std::collections::HashSet<SoundCue> = balance.keys().chain(confirmed.keys()).copied().collect();
    let diff: i64 = cues.iter().map(|c| (balance.get(c).copied().unwrap_or(0) - confirmed.get(c).copied().unwrap_or(0)).abs()).sum();
    assert!(diff <= unsettled, "viewer {}: played-minus-cancelled cues differ from the confirmed cues by {diff} ({unsettled} unsettled)", feed.viewer);
    (plays, cancels)
}

fn run_latency(latency: u32, jitter: u32, input_delay: u32) {
    for seed in [1u64, 2, 3] {
        let config = NetConfig { input_delay, ..NetConfig::latency(latency, jitter) };
        let (report, feeds) = play(seed, config);
        let [a, b] = &report.peers;
        let cues: Vec<(usize, usize)> = feeds.iter().map(check_cues).collect();
        eprintln!(
            "latency {latency}+{jitter} delay {input_delay} seed {seed}: {} frames, over {}; rollbacks {}/{} (max depth {}/{}, resimulated {}/{}); cues played/cancelled {:?}",
            report.frames,
            report.over,
            a.rollbacks,
            b.rollbacks,
            a.max_rollback,
            b.max_rollback,
            a.resimulated,
            b.resimulated,
            cues,
        );
        assert!(report.in_sync(), "seed {seed}: {:?} {:?}", report.divergence, report.panicked);
        assert!(report.over, "seed {seed}: the battle didn't end in {} frames", report.frames);
        if latency > input_delay {
            assert!(a.rollbacks > 0 && b.rollbacks > 0, "the latency should cause rollbacks");
        } else {
            assert_eq!((a.rollbacks, b.rollbacks), (0, 0), "no rollback without latency");
        }
    }
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
fn latency_10_with_input_delay() {
    run_latency(10, 2, 3);
}

/// The engine's own input record, with the custom screen's events riding
/// in player 0's input: record a stand-in battle's tick inputs, then play
/// them back through `Battle` as a rollback game.
#[test]
fn recorded_events_ride_in_the_inputs() {
    let mut g = start(7);
    let mut inputs = mashers(7);
    let mut record: Vec<TickInput> = Vec::new();
    while !g.is_over() {
        let buttons = [inputs(0, 0), inputs(1, 0)];
        let t = g.tick_input(buttons);
        g.battle.step(&t);
        record.push(t);
    }
    let shares = |p: usize, f: u32| {
        let t = &record[(f as usize).min(record.len() - 1)];
        Bn6Input { tick: t.players[p], events: if p == 0 { t.events.clone() } else { Default::default() } }
    };
    let final_digest = g.battle.digest();
    for latency in [3, 8] {
        let mut m = Match::new(&start(7).battle, NetConfig::latency(latency, 2));
        let report = m.run(&shares, &mut [(), ()], 30_000);
        assert!(report.in_sync() && report.over, "latency {latency}: {:?} {:?}", report.divergence, report.panicked);
        assert!(report.frames as usize >= record.len());
        for p in 0..2 {
            assert_eq!(m.confirmed_digests(p)[record.len() - 1], final_digest);
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
    b.battle = Battle::new(bn6_battle::RoundSetup { local_side: 1, ..netbattle(500, 5 ^ 0x1234_5678) });
    let mut inputs = mashers(5);
    let first_difference = (0..200u32).find(|_| {
        let i = [inputs(0, 0), inputs(1, 0)];
        a.advance(&i);
        b.advance(&i);
        a.battle.digest() != b.battle.digest()
    });
    assert!(first_difference.is_some_and(|f| f < 5), "{first_difference:?}");
}

/// Presentation hooks see every simulated frame once more per rollback,
/// and every confirmed frame once.
#[test]
fn confirmed_frames_are_reported_once_in_order() {
    struct Count {
        confirmed: Vec<u32>,
        simulated: u64,
        resimulated: u64,
    }
    impl<G: Game> Observer<G> for Count {
        fn simulated(&mut self, _: u32, _: &G, resim: bool) {
            if resim {
                self.resimulated += 1;
            } else {
                self.simulated += 1;
            }
        }
        fn confirmed(&mut self, frame: u32, _: &G) {
            self.confirmed.push(frame);
        }
    }
    let new = || Count { confirmed: Vec::new(), simulated: 0, resimulated: 0 };
    let mut counts = [new(), new()];
    let config = NetConfig { link: LinkConfig { latency: 4, jitter: 4 }, input_delay: 1, max_prediction: 12, seed: 99 };
    let report = Match::new(&start(4), config).run(mashers(4), &mut counts, 3000);
    assert!(report.in_sync());
    for (c, stats) in counts.iter().zip(&report.peers) {
        assert!(c.confirmed.iter().enumerate().all(|(i, &f)| f == i as u32));
        assert_eq!((c.simulated, c.resimulated), (stats.simulated, stats.resimulated));
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
        fn advance(&mut self, inputs: &[u16; 2]) {
            // Only every fourth A press gets through.
            let a = inputs[0] & bn6_battle::input::keys::A != 0;
            self.presses.set(self.presses.get() + a as u32);
            let held = if a && self.presses.get() % 4 != 0 { inputs[0] & !bn6_battle::input::keys::A } else { inputs[0] };
            self.game.advance(&[held, inputs[1]]);
        }
        fn digest(&self) -> u64 {
            self.game.digest()
        }
        fn is_over(&self) -> bool {
            self.game.is_over()
        }
        fn blank_input() -> u16 {
            0
        }
    }
    let leaky = || Leaky { game: start(1), presses: Rc::new(Cell::new(0)) };
    let mut m = Match::from_starts([leaky(), leaky()], leaky(), NetConfig::latency(4, 2));
    let report = m.run(mashers(1), &mut [(), ()], 30_000);
    let d = report.divergence.expect("the leak goes unnoticed");
    eprintln!("state outside the snapshot: out of sync from frame {} ({} rollbacks by then)", d.frame, report.peers[0].rollbacks);
    // Without latency there is no rollback, and nothing to notice.
    let mut m = Match::from_starts([leaky(), leaky()], leaky(), NetConfig::latency(0, 0));
    assert!(m.run(mashers(1), &mut [(), ()], 30_000).in_sync());
}

/// With a content feature (`luau`, `rust-content`), every battle above
/// runs the GunDelSol slice as content (docs/design/scripting.md), so these
/// tests are also the scripted slice under rollback.
#[test]
fn the_battles_run_the_featured_content() {
    let want = if cfg!(feature = "luau") {
        "luau"
    } else if cfg!(feature = "rust-content") {
        "rust"
    } else {
        "builtin"
    };
    assert_eq!(start(1).battle.content.runtime(), want);
}
