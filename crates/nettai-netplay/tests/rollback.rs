//! Synthetic netbattles under rollback: two MegaMen with fixed folders
//! and seeded random button mashing (which drives their custom screens
//! too), played by two getgud sessions over simulated links with various
//! latencies. At every advance, each peer's settled state must equal the
//! other's and a plain lockstep run's, the battle must end, and each
//! peer's sound must play every confirmed cue once.

use nettai_battle::cues::CueAction;
use nettai_battle::{Battle, SoundCue, TickInput};
use nettai_netplay::battle::{PlayerInput, CueFeed};
use nettai_netplay::network::LinkConfig;
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
/// dimming (the invisibility dimming chip). Side 1's: GunDelSols only, so
/// that neither side can cut in on a dimming with one of its own (not
/// implemented yet). The codes are the chips' own (A and *).
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

fn run_latency(latency: u32, jitter: u32, present_delay: u32) {
    for seed in [1u64, 2, 3] {
        let config = NetConfig { present_delay, ..NetConfig::latency(latency, jitter) };
        let (report, feeds) = play(seed, config);
        let [a, b] = &report.peers;
        let cues: Vec<(usize, usize)> = feeds.iter().map(check_cues).collect();
        eprintln!(
            "latency {latency}+{jitter} present delay {present_delay} seed {seed}: {} frames in {} wall frames, over {}; \
             rollbacks {}/{} (max depth {}/{}, resimulated {}/{}, speculated up to {}/{}); stalls {}/{}; cues played/cancelled {:?}",
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
            cues,
        );
        assert!(report.in_sync(), "seed {seed}: {:?} {:?}", report.divergence, report.panicked);
        assert!(report.over, "seed {seed}: the battle didn't end in {} frames", report.frames);
        if latency > present_delay {
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
fn latency_10_with_present_delay() {
    run_latency(10, 2, 3);
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
        assert!(report.in_sync() && report.over, "latency {latency}: {:?} {:?}", report.divergence, report.panicked);
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
/// settled frames in order, each once.
#[test]
fn observers_see_every_simulated_and_settled_frame() {
    struct Count {
        settled: Vec<u32>,
        simulated: u64,
        resimulated: u64,
        reached: u32,
    }
    impl<G> Observer<G> for Count {
        fn simulated(&mut self, frame: u32, _: &G) {
            if frame < self.reached {
                self.resimulated += 1;
            } else {
                assert_eq!(frame, self.reached, "frames are first simulated in order");
                self.reached += 1;
                self.simulated += 1;
            }
        }
        fn confirmed(&mut self, frames: u32, settled: &Battle) {
            assert!(self.settled.last().is_none_or(|&last| frames > last));
            assert!(frames <= self.reached);
            let _ = settled.digest();
            self.settled.push(frames);
        }
    }
    let new = || Count { settled: Vec::new(), simulated: 0, resimulated: 0, reached: 0 };
    let mut counts = [new(), new()];
    let config = NetConfig { link: LinkConfig { latency: 4, jitter: 4 }, present_delay: 1, ..NetConfig::latency(4, 4) };
    let report = Match::new(&start(4), config).run(mashers(4), &mut counts, 3000);
    assert!(report.in_sync());
    assert_eq!(report.frames, 3000);
    for ((c, stats), settled) in counts.iter().zip(&report.peers).zip(&report.settled) {
        assert_eq!(c.settled, settled.iter().map(|s| s.0).collect::<Vec<_>>());
        assert_eq!((c.simulated, c.resimulated), (stats.simulated, stats.resimulated));
        assert!(stats.rollbacks > 0);
    }
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

/// Every battle above runs the test content's scripts (GunDelSol, the
/// eraser navi chip, the buster: docs/design/scripting.md), so these tests
/// are also the scripted content under rollback.
#[test]
fn the_battles_run_the_content_scripts() {
    let battle = start(1).battle;
    let runtime = nettai_battle::behavior::Behaviors::for_content(&battle.content).unwrap();
    assert_eq!(runtime.runtime(), "luau");
}
