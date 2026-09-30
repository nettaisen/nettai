//! What rollback netplay needs from the engine (docs/design/rollback.md):
//!
//! - one record of everything from outside the simulation that a tick
//!   consumes, [`TickInput`], and [`Battle::step`] to run a tick on it;
//! - snapshots: [`Battle::save_state`] and [`Battle::load_state`] (a
//!   battle is plain data, so a snapshot is a copy of it);
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
#[derive(Clone, Debug)]
pub struct Snapshot(Battle);

impl Snapshot {
    /// The saved battle.
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
        Snapshot(self.clone())
    }

    /// Save into an existing snapshot.
    pub fn save_state_into(&self, snapshot: &mut Snapshot) {
        snapshot.0.clone_from(self);
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
    use crate::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, effects};

    fn battle() -> Battle {
        let stats = NaviStats { hp: 300, max_hp: 300, max_base_hp: 300, ..NaviStats::default() };
        Battle::new(RoundSetup {
            settings: BattleSettings { effects: effects::LINK, ..crate::data::BATTLE_SETTINGS[0] },
            navi_stats: [stats; 2],
            rng: 0x2468_ACE0,
            local_side: 0,
            score: SetScore::default(),
            later_stages: Default::default(),
            low_hp_music_latched: false,
            sp_times: Default::default(),
            players: Default::default(),
            link_delay: 0,
        })
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
    fn the_digest_sees_simulation_state_but_not_presentation() {
        let mut b = battle();
        for f in 0..30 {
            b.step(&input(f));
        }
        let d = b.digest();
        assert_eq!(b.clone().digest(), d, "a copy digests the same");
        let mut c = b.clone();
        c.play_sound(crate::sound::SoundId(0x94));
        c.banner.id = Some(crate::data::BannerId(0x4C));
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
