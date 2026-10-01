//! One peer's live battle as a getgud [`World`].
//!
//! getgud speaks of one local player and remote slots; the engine of two
//! sides. [`BattleWorld`] knows its player's side and puts `local` and the
//! one remote into their places ([`Game::step`] takes both inputs by
//! side). A [`Game`] is a battle plus how it steps: [`Battle`] on the
//! engine's input record (`bn6`), or the stand-in battle on buttons alone
//! (`standin`).
//!
//! Saved states are the engine's snapshots ([`Battle::save_state`]), with
//! the tick they are parked at. getgud never shows the host its world, so
//! what only the world sees, every tick it simulates and every rewind, goes
//! to an [`Observer`] the world is given (sound, checks); the host tells
//! the same observer what settles.

use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use bn6_battle::{Battle, Snapshot};
use getgud::{Session, SessionParams, World};

/// A two-player battle that both peers simulate: the next state is a
/// function of the state and both players' inputs only. Its whole state is
/// the battle ([`Game::battle`]), which is what a snapshot saves.
pub trait Game {
    /// One player's input for one tick. `Default` is the input assumed
    /// before any of that player's has arrived.
    type Input: Clone + PartialEq + Default + Send + 'static;

    /// Simulate one tick on both players' inputs, by side.
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

/// What happens to one peer's simulation. Frames are numbered from 0:
/// frame `f` is the `f`-th tick, and "the state after frame `f`" is the
/// battle once `f + 1` ticks have run.
///
/// The world calls [`rolled_back`](Observer::rolled_back) and
/// [`simulated`](Observer::simulated) as getgud drives it; the host calls
/// [`confirmed`](Observer::confirmed) after each advance that settled
/// frames (getgud's `Advance::confirmed` rows).
pub trait Observer<G> {
    /// The world went back to the state after `frame` ticks: frames
    /// `frame` and later are about to be simulated again.
    fn rolled_back(&mut self, _frame: u32) {}

    /// Frame `frame` was simulated, the first time or again after a
    /// rollback. `game` is the state after it.
    fn simulated(&mut self, _frame: u32, _game: &G) {}

    /// Frames before `frames` are settled: confirmed, never simulated
    /// again. `settled` is the settled state (after `frames` ticks). The
    /// last simulation of each of those frames that the observer saw was
    /// on the confirmed inputs.
    fn confirmed(&mut self, _frames: u32, _settled: &Battle) {}
}

impl<G> Observer<G> for () {}

impl<G, O: Observer<G> + ?Sized> Observer<G> for &mut O {
    fn rolled_back(&mut self, frame: u32) {
        (**self).rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G) {
        (**self).simulated(frame, game);
    }
    fn confirmed(&mut self, frames: u32, settled: &Battle) {
        (**self).confirmed(frames, settled);
    }
}

/// An observer shared by the world (which getgud owns) and the host.
impl<G, O: Observer<G> + ?Sized> Observer<G> for &RefCell<O> {
    fn rolled_back(&mut self, frame: u32) {
        self.borrow_mut().rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G) {
        self.borrow_mut().simulated(frame, game);
    }
    fn confirmed(&mut self, frames: u32, settled: &Battle) {
        self.borrow_mut().confirmed(frames, settled);
    }
}

impl<G, O: Observer<G> + ?Sized> Observer<G> for Rc<RefCell<O>> {
    fn rolled_back(&mut self, frame: u32) {
        self.borrow_mut().rolled_back(frame);
    }
    fn simulated(&mut self, frame: u32, game: &G) {
        self.borrow_mut().simulated(frame, game);
    }
    fn confirmed(&mut self, frames: u32, settled: &Battle) {
        self.borrow_mut().confirmed(frames, settled);
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
/// simulates and every rewind.
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
    /// The same, telling `observer` what it simulates.
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
        self.game.step(inputs);
        let frame = self.tick;
        self.tick += 1;
        self.observer.simulated(frame, &self.game);
        Ok(())
    }

    fn save(&mut self) -> Result<BattleState, Infallible> {
        Ok(self.state())
    }

    /// getgud loads the settled state to re-simulate from it. When the
    /// world is parked there already (nothing was speculated past it), it
    /// is that state: getgud saved it from this world, or promoted a
    /// snapshot this world made, and nothing has stepped since. Then
    /// there's nothing to restore and nothing is rolled back.
    fn load(&mut self, state: &BattleState) -> Result<(), Infallible> {
        if state.tick != self.tick {
            self.game.battle_mut().load_state(&state.snapshot);
            self.tick = state.tick;
            self.observer.rolled_back(state.tick);
        }
        Ok(())
    }

    fn predict(&self, last: &G::Input) -> G::Input {
        G::predict(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standin::{StandInBattle, folder, netbattle};
    use bn6_battle::content::testing;

    fn standin() -> StandInBattle {
        use testing::{SUN_GUN_1, SUN_GUN_3};
        let c = testing::content();
        let f = || folder(&c, &[(SUN_GUN_3, 0), (SUN_GUN_1, 0)]);
        StandInBattle::new(Battle::new(netbattle(&c, bn6_battle::content::testing::LINK_BATTLE, 300, 0x1357, [f(), f()]), c))
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
    }

    /// Each world puts its own player's input on its own side: the two
    /// peers' worlds, stepped with each other's `local` and `remote`, stay
    /// equal.
    #[test]
    fn local_and_remote_go_to_their_sides() {
        use bn6_battle::input::keys;
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

    /// Loading the tick the world is parked at restores nothing and rolls
    /// nothing back; loading an earlier one does both.
    #[test]
    fn a_load_rewinds_only_to_an_earlier_tick() {
        let mut w = BattleWorld::with_observer(standin(), 0, Seen::default());
        w.step(&0, &[0]).unwrap();
        let first = w.save().unwrap();
        w.load(&first).unwrap();
        w.step(&0, &[0]).unwrap();
        let second = w.save().unwrap();
        w.load(&first).unwrap();
        assert_eq!(w.tick(), 1);
        w.step(&0, &[0]).unwrap();
        assert_eq!(w.game().battle().digest(), second.battle().digest());
        assert_eq!(w.observer.0, ["frame 0", "frame 1", "back to 1", "frame 1"]);
    }
}
