//! What rollback netplay needs from the engine (docs/design/rollback.md):
//!
//! - one record of everything from outside the simulation that a tick
//!   consumes, [`TickInput`], and [`Battle::step`] to run a tick on it;
//! - snapshots: [`Battle::save_state`] and [`Battle::load_state`] (a
//!   battle is plain data and `Send`, so a snapshot is a copy of it);
//! - a digest of the simulation state for desync detection,
//!   [`Battle::digest`] (in `digest`);
//! - what each viewer is shown when the peers present one simulation from
//!   two sides (in `perspective`).
//!
//! The simulation is a pure function of its setup and the tick inputs:
//! it keeps no state outside `Battle`, reads no clock, does no I/O, and
//! uses no floats, statics, interior mutability, hash-map iteration or
//! addresses.

use crate::battle::{Battle, TickEvents};
use crate::behavior::Behaviors;
use crate::input::PlayerTick;

/// Everything from outside the simulation that one tick consumes. Two
/// battles started from the same setup and stepped with the same inputs
/// are in the same state.
///
/// Each player's share is their buttons ([`PlayerTick`]); both players'
/// custom screens run in the engine from them. The [`TickEvents`] are the
/// link session closing at the end of the round. In netplay the events belong to the
/// frame's input record like the buttons do: both peers must step the
/// frame with the same events (the netplay layer carries them in a
/// player's input).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TickInput {
    /// By side.
    pub players: [PlayerTick; 2],
    pub events: TickEvents,
}

// A battle, its snapshots and its inputs go to any thread and are shared
// between threads: a netplay session keeps snapshots in its buffers and may
// run on a thread of its own (getgud requires its states and inputs to be
// `Send`). This fails to compile if a battle ever holds something that
// can't, like an `Rc`, a `RefCell` or the content's runtime.
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<Battle>();
    send_sync::<Snapshot>();
    send_sync::<TickInput>();
    send_sync::<PlayerTick>();
    send_sync::<TickEvents>();
};

/// A saved battle: everything needed to resume the simulation exactly.
///
/// A battle is plain data, `Send` and `Sync` (its content is an `Arc`, and
/// the content's runtime is not part of it: each thread keeps its own, see
/// `behavior`), so a snapshot is a copy a netplay layer can keep wherever
/// it needs to (getgud requires its saved states to be `Send`).
///
/// The battle is boxed: a netplay layer moves snapshots around (into its
/// buffers, out of them) more often than it makes them, and a battle is
/// several KB inline.
#[derive(Clone, Debug)]
pub struct Snapshot(Box<Battle>);

impl Snapshot {
    /// The saved battle, to read.
    pub fn battle(&self) -> &Battle {
        &self.0
    }
}

impl Battle {
    /// One tick on an input record.
    pub fn step(&mut self, input: &TickInput) {
        self.tick(&input.players, input.events.clone());
    }

    /// One tick on an input record, with `runtime` running the content (a
    /// runtime loaded with particular options: `behavior::with_runtime`).
    pub fn step_with(&mut self, runtime: &Behaviors, input: &TickInput) {
        crate::behavior::with_runtime(runtime, || self.step(input));
    }

    /// Save the whole simulation state.
    pub fn save_state(&self) -> Snapshot {
        Snapshot(Box::new(self.clone()))
    }

    /// Save into an existing snapshot.
    pub fn save_state_into(&self, snapshot: &mut Snapshot) {
        Battle::clone_from(&mut snapshot.0, self);
    }

    /// Go back to a saved state. The next tick continues from it exactly
    /// as the saved battle would have.
    pub fn load_state(&mut self, snapshot: &Snapshot) {
        self.clone_from(&snapshot.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::keys;
    use crate::content::testing;

    fn battle() -> Battle {
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(300));
        setup.rng = 0x2468_ACE0;
        Battle::new(setup, testing::content())
    }

    fn input(frame: u32) -> TickInput {
        // Walk about: a direction every few frames.
        let dirs = [keys::UP, keys::RIGHT, keys::DOWN, keys::LEFT, 0];
        let held = |p: u32| dirs[((frame / 7 + p * 3) % 5) as usize];
        TickInput { players: [PlayerTick::buttons(held(0)), PlayerTick::buttons(held(1))], events: TickEvents::default() }
    }

    #[test]
    fn a_restored_battle_replays_identically() {
        let mut b = battle();
        for f in 0..40 {
            b.step(&input(f));
        }
        let snapshot = b.save_state();
        let digest = b.digest();
        let mut ahead = Vec::new();
        for f in 40..120 {
            b.step(&input(f));
            ahead.push(b.digest());
        }
        // Mispredict, then roll back and replay the real inputs.
        b.load_state(&snapshot);
        assert_eq!(b.digest(), digest);
        for f in 40..80 {
            b.step(&input(f + 3));
        }
        b.load_state(&snapshot);
        let again: Vec<u64> = (40..120).map(|f| {
            b.step(&input(f));
            b.digest()
        }).collect();
        assert_eq!(again, ahead);
    }

    #[test]
    fn a_battle_and_its_snapshots_are_send_and_step_on_any_thread() {
        fn send<T: Send>(_: &T) {}
        let mut b = battle();
        send(&b);
        for f in 0..20 {
            b.step(&input(f));
        }
        let snapshot = b.save_state();
        send(&snapshot);
        assert_eq!(snapshot.battle().digest(), b.digest());
        let mut refilled = battle().save_state();
        b.save_state_into(&mut refilled);
        assert_eq!(refilled.battle().digest(), b.digest());
        // A snapshot crosses threads and comes back unchanged.
        let snapshot = std::thread::spawn(move || snapshot).join().unwrap();
        let ahead: Vec<u64> = (20..60).map(|f| {
            b.step(&input(f));
            b.digest()
        }).collect();
        // A battle steps on another thread (with that thread's runtime)
        // exactly as on this one.
        let mut moved = battle();
        moved.load_state(&snapshot);
        let there: Vec<u64> = std::thread::spawn(move || {
            (20..60).map(|f| {
                moved.step(&input(f));
                moved.digest()
            }).collect()
        }).join().unwrap();
        assert_eq!(there, ahead);
        b.load_state(&snapshot);
        let again: Vec<u64> = (20..60).map(|f| {
            b.step(&input(f));
            b.digest()
        }).collect();
        assert_eq!(again, ahead);
    }

    #[test]
    fn the_digest_sees_simulation_state_but_not_presentation() {
        let mut b = battle();
        for f in 0..30 {
            b.step(&input(f));
        }
        let d = b.digest();
        assert_eq!(b.clone().digest(), d, "a copy digests the same");
        let mut c = b.clone();
        c.play_sound(crate::sound::SoundId(0x94));
        c.banner.id = Some(crate::content::BannerId(0x4C));
        assert_eq!(c.digest(), d, "sound cues and the banner id are presentation");
        let mut c = b.clone();
        c.rng.next();
        assert_ne!(c.digest(), d);
        let mut c = b.clone();
        let r = c.objects.in_order().next().unwrap();
        c.objects.get_mut(r).timer ^= 1;
        assert_ne!(c.digest(), d);
    }

    /// The panels attacked during a tick (`Battle::attacked_panels`) are a
    /// look at the tick for a reader of the battle: the buster's one-tick
    /// hit regions show there after the tick that presented them, and the
    /// log is neither digested nor read by the simulation (a battle whose
    /// log is scrambled after every tick goes on, digest for digest, as one
    /// left alone).
    #[test]
    fn the_attacked_panels_are_no_part_of_the_simulation() {
        // (On the custom screen START and A, to OK; in the fight B tapped
        // every 8 frames: buster shots, each a hit region.)
        let shoot = |b: &Battle, frame: u32| {
            let held = match (b.round.mode == crate::battle::mode::CUSTOM, frame % 8) {
                (true, 0) => keys::START,
                (true, 4) => keys::A,
                (false, 0 | 1) => keys::B,
                _ => 0,
            };
            TickInput { players: [PlayerTick::buttons(held); 2], events: TickEvents::default() }
        };
        // (Navis with the test content's buster.)
        let armed = || {
            let mut stats = testing::stats(300);
            stats.weapons.buster = testing::weapon("megaman/buster");
            let mut setup = testing::round_setup(testing::LINK_BATTLE, stats);
            setup.rng = 0x2468_ACE0;
            Battle::new(setup, testing::content())
        };
        let (mut b, mut scrambled) = (armed(), armed());
        let (mut seen, mut dealt) = ([false; 2], [false; 2]);
        for f in 0..900 {
            let i = shoot(&b, f);
            b.step(&i);
            scrambled.step(&i);
            assert_eq!(scrambled.digest(), b.digest(), "tick {f}");
            for side in 0..2u8 {
                seen[side as usize] |= b.attacked_panels(side) != 0;
                // (Only the playable panels' bits.)
                assert_eq!(b.attacked_panels(side) & !0x7E_7E7E_00, 0, "tick {f}");
                // (A panel's damage only where it was attacked.)
                for p in 0..32 {
                    let attacked = b.attacked_panels(side) & (1 << p) != 0;
                    assert!(attacked || b.attack_damage(side, p) == 0, "tick {f}: panel {p}");
                    dealt[side as usize] |= attacked && b.attack_damage(side, p) > 0;
                }
            }
            let d = scrambled.digest();
            scrambled.attacked.panels = [u64::MAX, 0x1234];
            scrambled.attacked.damage = [[0xFFFF; 32], [7; 32]];
            assert_eq!(scrambled.digest(), d, "tick {f}: the log isn't digested");
        }
        assert_eq!(seen, [true, true], "both sides' shots were logged");
        assert_eq!(dealt, [true, true], "with their damage");
    }
}
