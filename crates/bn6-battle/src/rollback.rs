//! What rollback netplay needs from the engine (docs/design/rollback.md):
//!
//! - one record of everything from outside the simulation that a tick
//!   consumes, [`TickInput`], and [`Battle::step`] to run a tick on it;
//! - snapshots: [`Battle::save_state`] and [`Battle::load_state`] (a
//!   battle is plain data, so a snapshot is a copy of it, less the
//!   behaviors handle, which makes it `Send`);
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
/// link session closing at the end of the round, and, only when checking
/// against a recording that lacks a player's folder, that player's
/// recorded custom-screen results. In netplay the events belong to the
/// frame's input record like the buttons do: both peers must step the
/// frame with the same events (the netplay layer carries them in a
/// player's input).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TickInput {
    /// By side.
    pub players: [PlayerTick; 2],
    pub events: TickEvents,
}

/// A saved battle: everything needed to resume the simulation exactly.
///
/// A snapshot is plain data and `Send`, so a netplay layer can keep it
/// wherever it needs to (getgud requires its saved states to be `Send`).
/// It holds the battle without its behaviors handle: that is shared code
/// on the battle's thread, not state (an `Rc`), and
/// [`Battle::load_state`] keeps the live battle's own. So the saved
/// battle can be read (drawn, digested, compared) but not stepped:
/// restore it into a live battle to go on.
///
/// The battle is boxed: a netplay layer moves snapshots around (into its
/// buffers, out of them) more often than it makes them, and a battle is
/// several KB inline.
#[derive(Clone, Debug)]
pub struct Snapshot(Box<Battle>);

// SAFETY: a `Battle` is `Send` but for its behaviors handle, which holds
// an `Rc`. A snapshot never holds one: the field is private, and every
// way to make or fill a snapshot (`save_state`, `save_state_into`)
// detaches the handle, leaving `Behaviors::none()`, which holds nothing.
// `snapshot_holds_only_send_state` below stops compiling if any other
// part of a battle stops being `Send`.
unsafe impl Send for Snapshot {}

/// What makes [`Snapshot`]'s `Send` sound: every part of a battle but the
/// behaviors handle is `Send`. The destructuring has no `..`, so a new
/// field doesn't compile until it is listed here.
#[allow(dead_code)]
fn snapshot_holds_only_send_state(battle: Battle) {
    fn send<T: Send>(_: T) {}
    let Battle {
        content,
        setup,
        stats,
        cross_stats,
        rng,
        round,
        fight,
        gauge,
        banner,
        paused,
        inputs,
        hands,
        transform_requests,
        turn_transforms,
        transform_seq,
        custom_reversion,
        beast_out_used,
        crossed,
        bug_frags,
        navi_levels,
        objects,
        actors,
        collision,
        field,
        fade,
        fadein_queue,
        damage_carry,
        custom,
        link,
        sides,
        side_stats,
        linked,
        dimming,
        sound,
        outcome,
        // Detached in every snapshot (see above).
        behaviors: _,
    } = battle;
    send(content);
    send(setup);
    send(stats);
    send(cross_stats);
    send(rng);
    send(round);
    send(fight);
    send(gauge);
    send(banner);
    send(paused);
    send(inputs);
    send(hands);
    send(transform_requests);
    send(turn_transforms);
    send(transform_seq);
    send(custom_reversion);
    send(beast_out_used);
    send(crossed);
    send(bug_frags);
    send(navi_levels);
    send(objects);
    send(actors);
    send(collision);
    send(field);
    send(fade);
    send(fadein_queue);
    send(damage_carry);
    send(custom);
    send(link);
    send(sides);
    send(side_stats);
    send(linked);
    send(dimming);
    send(sound);
    send(outcome);
}

impl Snapshot {
    /// The saved battle, to read (its behaviors handle is detached: see
    /// the type's docs).
    pub fn battle(&self) -> &Battle {
        &self.0
    }
}

impl Battle {
    /// One tick on an input record.
    pub fn step(&mut self, input: &TickInput) {
        self.tick(&input.players, input.events.clone());
    }

    /// Save the whole simulation state.
    pub fn save_state(&self) -> Snapshot {
        let mut saved = Box::new(self.clone());
        saved.behaviors = Behaviors::none();
        Snapshot(saved)
    }

    /// Save into an existing snapshot.
    pub fn save_state_into(&self, snapshot: &mut Snapshot) {
        Battle::clone_from(&mut snapshot.0, self);
        snapshot.0.behaviors = Behaviors::none();
    }

    /// Go back to a saved state. The next tick continues from it exactly
    /// as the saved battle would have. The battle keeps its own behaviors
    /// handle (the content is the same: `RoundSetup::content` is part of
    /// the saved state).
    pub fn load_state(&mut self, snapshot: &Snapshot) {
        let behaviors = std::mem::take(&mut self.behaviors);
        self.clone_from(&snapshot.0);
        self.behaviors = behaviors;
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
        TickInput { players: [PlayerTick { held: held(0) }, PlayerTick { held: held(1) }], events: TickEvents::default() }
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
    fn a_snapshot_is_send_and_the_restored_battle_keeps_its_scripts() {
        fn send<T: Send>(_: &T) {}
        let mut b = battle();
        assert_eq!(b.behaviors.runtime(), "luau");
        for f in 0..20 {
            b.step(&input(f));
        }
        let snapshot = b.save_state();
        send(&snapshot);
        // The saved battle is for reading: it holds no handle.
        assert_eq!(snapshot.battle().behaviors.runtime(), "none");
        assert_eq!(snapshot.battle().digest(), b.digest());
        let mut refilled = battle().save_state();
        b.save_state_into(&mut refilled);
        assert_eq!(refilled.battle().behaviors.runtime(), "none");
        // A snapshot crosses threads and comes back unchanged.
        let snapshot = std::thread::spawn(move || snapshot).join().unwrap();
        let ahead: Vec<u64> = (20..60).map(|f| {
            b.step(&input(f));
            b.digest()
        }).collect();
        b.load_state(&snapshot);
        assert_eq!(b.behaviors.runtime(), "luau", "the live battle keeps its scripts");
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
}
