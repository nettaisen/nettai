//! One peer's live battle as a getgud [`World`].
//!
//! getgud speaks of one local player and remote slots; the engine of two
//! sides. [`BattleWorld`] knows its player's side and puts `local` and the
//! one remote into their places ([`Game::step`] takes both inputs by
//! side). A [`Game`] is a battle plus how it steps: [`Battle`] on the
//! engine's input record (`battle`), or the stand-in battle on buttons alone
//! (`standin`).
//!
//! Saved states are the engine's snapshots ([`Battle::save_state`]), with
//! the tick they are parked at. The world tells its [`Observer`] everything
//! that happens to its simulation: every tick it simulates, every rewind,
//! and every tick that settles (getgud tells the world, `World::settled`).
//! The observer is the world's own; the host reads it through the session
//! (`Session::world`). A session on a battle world is `Send` when its
//! observer is, so it can run on a network thread.
//!
//! A tick that panics (a content error, or a state the original can't go on
//! from) stops the battle instead ([`step_game`]), the same way on every
//! peer.

use std::any::Any;
use std::convert::Infallible;
use std::panic::{AssertUnwindSafe, catch_unwind};

use getgud::{Confirmed, Session, SessionParams, World};
use nettai_battle::{Battle, Snapshot};

/// A two-player battle that both peers simulate: the next state is a
/// function of the state and both players' inputs only. Its whole state is
/// the battle ([`Game::battle`]), which is what a snapshot saves.
pub trait Game {
    /// One player's input for one tick. `Default` is the input assumed
    /// before any of that player's has arrived.
    type Input: Clone + PartialEq + Default + Send + 'static;

    /// Simulate one tick on both players' inputs, by side. A panic stops
    /// the battle when a peer steps it ([`step_game`]).
    fn step(&mut self, inputs: [&Self::Input; 2]);

    /// The battle: the whole state.
    fn battle(&self) -> &Battle;

    /// The same, to restore a snapshot into.
    fn battle_mut(&mut self) -> &mut Battle;

    /// The input to assume for a tick whose input hasn't arrived, from the
    /// player's latest one.
    fn predict(last: &Self::Input) -> Self::Input {
        last.clone()
    }
}

/// Step `game` one tick as a peer does: a tick that panics stops the battle
/// with the panic's message ([`Battle::fail`], `RoundEnd::Error`) rather
/// than unwinding into the netcode. The panicking tick ran the same code on
/// the same state up to the panic on every peer, so a tick that fails on
/// confirmed inputs ends the match on both peers alike, and one that fails
/// on predicted inputs is rolled back like any speculation.
///
/// (The panic still reaches the panic hook, which prints it unless the host
/// installs a quieter one.)
pub fn step_game<G: Game>(game: &mut G, inputs: [&G::Input; 2]) {
    if let Err(panic) = catch_unwind(AssertUnwindSafe(|| game.step(inputs))) {
        game.battle_mut().fail(panic_message(&*panic));
    }
}

/// A panic's message, from its payload.
fn panic_message(panic: &(dyn Any + Send)) -> String {
    match panic.downcast_ref::<String>() {
        Some(s) => s.clone(),
        None => panic.downcast_ref::<&str>().map_or("a panic without a message", |s| s).to_string(),
    }
}

/// What happens to one peer's simulation. Frames are numbered from 0:
/// frame `f` is the `f`-th tick, and "the state after frame `f`" is the
/// battle once `f + 1` ticks have run.
///
/// The world calls all three as getgud drives it.
pub trait Observer<G> {
    /// The world went back to the state after `frame` ticks: frames
    /// `frame` and later are about to be simulated again.
    fn rolled_back(&mut self, _frame: u32) {}

    /// Frame `frame` was simulated, the first time or again after a
    /// rollback. `game` is the state after it.
    fn simulated(&mut self, _frame: u32, _game: &G) {}

    /// Frame `frame` settled: confirmed, never simulated again. Frames
    /// settle in order, each once, and the last [`simulated`] of a frame
    /// before it settles was on its confirmed inputs. `settled` is the
    /// state after it where getgud kept one: a frame whose speculation was
    /// promoted, or the last frame of a re-simulation (`None` for the frames
    /// re-simulated before that one).
    ///
    /// [`simulated`]: Observer::simulated
    fn confirmed(&mut self, _frame: u32, _settled: Option<&Battle>) {}
}

impl<G> Observer<G> for () {}

impl<G, O: Observer<G> + ?Sized> Observer<G> for &mut O {
    fn rolled_back(&mut self, frame: u32) {
        (**self).rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G) {
        (**self).simulated(frame, game);
    }
    fn confirmed(&mut self, frame: u32, settled: Option<&Battle>) {
        (**self).confirmed(frame, settled);
    }
}

/// A saved battle and the tick it is parked at (the number of ticks run
/// since the session started): getgud's `State`.
#[derive(Clone, Debug)]
pub struct BattleState {
    tick: u32,
    snapshot: Snapshot,
}

impl BattleState {
    /// Ticks run since the session started.
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// The battle, to present or check. It can't be stepped: restore it
    /// into a live battle for that (see [`Snapshot`]).
    pub fn battle(&self) -> &Battle {
        self.snapshot.battle()
    }
}

/// One peer's live battle: getgud's [`World`]. It steps the game on its
/// player's input (`local`) and the other player's (the one remote slot),
/// each in its side's place, and tells its observer every tick it
/// simulates, every rewind and every tick that settles.
pub struct BattleWorld<G: Game, O: Observer<G> = ()> {
    game: G,
    /// The side this peer's player plays (0 or 1).
    side: usize,
    /// Ticks run since the session started: where the world is parked.
    tick: u32,
    observer: O,
}

impl<G: Game> BattleWorld<G> {
    /// The world of the peer playing `side`, starting from `game`.
    pub fn new(game: G, side: usize) -> BattleWorld<G> {
        BattleWorld::with_observer(game, side, ())
    }
}

impl<G: Game, O: Observer<G>> BattleWorld<G, O> {
    /// The same, telling `observer` what happens to its simulation.
    pub fn with_observer(game: G, side: usize, observer: O) -> BattleWorld<G, O> {
        assert!(side < 2, "a battle has sides 0 and 1");
        BattleWorld { game, side, tick: 0, observer }
    }

    pub fn side(&self) -> usize {
        self.side
    }

    /// Ticks run since the session started.
    pub fn tick(&self) -> u32 {
        self.tick
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn observer(&self) -> &O {
        &self.observer
    }

    /// The observer, to change (to take the sound actions it collected, say).
    /// Through the session, `Session::world_mut` costs the session a restore
    /// before it next steps the world: take what a frame needs through
    /// `Session::world` where reading will do.
    pub fn observer_mut(&mut self) -> &mut O {
        &mut self.observer
    }

    /// The state the world is parked at.
    pub fn state(&self) -> BattleState {
        BattleState { tick: self.tick, snapshot: self.game.battle().save_state() }
    }

    /// A getgud session on this world, from where it is parked, showing
    /// frames `present_delay` ticks behind the newest local input. The
    /// world must be parked at tick 0: a session counts ticks from its
    /// start.
    pub fn session(self, present_delay: u32) -> Session<Self> {
        assert_eq!(self.tick, 0, "a session starts at tick 0");
        Session::new(SessionParams {
            present_delay,
            initial_remotes: vec![G::Input::default()],
            initial_state: self.state(),
            world: self,
        })
    }
}

impl<G: Game, O: Observer<G>> World for BattleWorld<G, O> {
    type Input = G::Input;
    type State = BattleState;
    type Error = Infallible;

    fn step(&mut self, local: &G::Input, remotes: &[G::Input]) -> Result<(), Infallible> {
        let [remote] = remotes else { panic!("a battle has one remote player, not {}", remotes.len()) };
        let inputs = if self.side == 0 { [local, remote] } else { [remote, local] };
        step_game(&mut self.game, inputs);
        let frame = self.tick;
        self.tick += 1;
        self.observer.simulated(frame, &self.game);
        Ok(())
    }

    fn save(&mut self) -> Result<BattleState, Infallible> {
        Ok(self.state())
    }

    /// getgud loads only to move the world (never the tick it is parked
    /// at), so a load is always a rewind.
    fn load(&mut self, state: &BattleState) -> Result<(), Infallible> {
        self.game.battle_mut().load_state(&state.snapshot);
        self.tick = state.tick;
        self.observer.rolled_back(state.tick);
        Ok(())
    }

    fn predict(&self, last: &G::Input) -> G::Input {
        G::predict(last)
    }

    fn settled(&mut self, row: &Confirmed<'_, Self>) {
        self.observer.confirmed(row.tick, row.state.map(BattleState::battle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standin::{StandInBattle, folder, netbattle};
    use nettai_battle::content::testing;

    fn standin() -> StandInBattle {
        use testing::{SUN_GUN_1, SUN_GUN_3};
        let c = testing::content();
        let f = || folder(&c, &[(SUN_GUN_3, 0), (SUN_GUN_1, 0)]);
        StandInBattle::new(Battle::new(netbattle(&c, nettai_battle::content::testing::LINK_BATTLE, 300, 0x1357, [f(), f()]), c))
    }

    #[derive(Default)]
    struct Seen(Vec<String>);

    impl<G> Observer<G> for Seen {
        fn rolled_back(&mut self, frame: u32) {
            self.0.push(format!("back to {frame}"));
        }
        fn simulated(&mut self, frame: u32, _: &G) {
            self.0.push(format!("frame {frame}"));
        }
        fn confirmed(&mut self, frame: u32, settled: Option<&Battle>) {
            self.0.push(format!("settled {frame}{}", if settled.is_some() { " with its state" } else { "" }));
        }
    }

    /// Each world puts its own player's input on its own side: the two
    /// peers' worlds, stepped with each other's `local` and `remote`, stay
    /// equal.
    #[test]
    fn local_and_remote_go_to_their_sides() {
        use nettai_battle::input::keys;
        let mut a = BattleWorld::new(standin(), 0);
        let mut b = BattleWorld::new(standin(), 1);
        let mut side_by_side = standin();
        for f in 0..120u16 {
            let (p0, p1) = (if f % 9 < 4 { keys::A } else { keys::LEFT }, if f % 7 < 2 { keys::UP } else { 0 });
            a.step(&p0, &[p1]).unwrap();
            b.step(&p1, &[p0]).unwrap();
            side_by_side.step([&p0, &p1]);
        }
        assert_eq!(a.game().battle().digest(), side_by_side.battle().digest());
        assert_eq!(b.game().battle().digest(), side_by_side.battle().digest());
    }

    /// A load rewinds and says so; the steps after it continue from the
    /// loaded state exactly.
    #[test]
    fn a_load_rewinds() {
        let mut w = BattleWorld::with_observer(standin(), 0, Seen::default());
        w.step(&0, &[0]).unwrap();
        let first = w.save().unwrap();
        w.step(&0, &[0]).unwrap();
        let second = w.save().unwrap();
        w.load(&first).unwrap();
        assert_eq!(w.tick(), 1);
        w.step(&0, &[0]).unwrap();
        assert_eq!(w.game().battle().digest(), second.battle().digest());
        assert_eq!(w.observer().0, ["frame 0", "frame 1", "back to 1", "frame 1"]);
    }

    /// Through a session, the world's observer hears every frame simulated,
    /// every rewind and every frame settled, from the world alone: here
    /// with the remote's input two ticks late and changing, so frames are
    /// simulated again.
    #[test]
    fn the_observer_hears_everything_from_the_world() {
        let mut s = BattleWorld::with_observer(standin(), 0, Seen::default()).session(0);
        let remote = |t: u32| if t % 6 < 3 { 0 } else { nettai_battle::input::keys::UP };
        for t in 0..12u32 {
            if t >= 2 {
                s.add_remote_input(0, remote(t - 2), 0);
            }
            let _ = s.advance(0).unwrap();
        }
        let seen = &s.world().observer().0;
        let settled: Vec<&String> = seen.iter().filter(|e| e.starts_with("settled")).collect();
        assert_eq!(settled.len() as u32, s.settled_tick());
        assert!(seen.iter().any(|e| e.starts_with("back to")), "{seen:?}");
        assert!(settled.iter().any(|e| e.ends_with("with its state")), "{seen:?}");
    }

    /// A tick that panics stops the battle (`RoundEnd::Error`) instead of
    /// unwinding into the session; the battle doesn't tick again.
    #[test]
    fn a_panicking_tick_stops_the_battle() {
        #[derive(Clone)]
        struct Brittle(StandInBattle);
        impl Game for Brittle {
            type Input = u16;
            fn step(&mut self, inputs: [&u16; 2]) {
                self.0.step(inputs);
                assert!(*inputs[0] != 0xBAD, "a bad input");
            }
            fn battle(&self) -> &Battle {
                &self.0.battle
            }
            fn battle_mut(&mut self) -> &mut Battle {
                &mut self.0.battle
            }
        }
        let quiet = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let mut w = BattleWorld::new(Brittle(standin()), 0);
        w.step(&0, &[0]).unwrap();
        w.step(&0xBAD, &[0]).unwrap();
        std::panic::set_hook(quiet);
        let stopped = w.game().battle().digest();
        assert_eq!(w.game().battle().round_end(), Some(&nettai_battle::RoundEnd::Error("a bad input".into())));
        w.step(&0, &[0]).unwrap();
        assert_eq!(w.game().battle().digest(), stopped, "a stopped battle doesn't tick");
        assert_eq!(w.tick(), 3);
    }
}
