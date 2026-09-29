//! Rollback netplay over the engine, GGPO style (docs/design/scripting.md
//! §Rollback).
//!
//! Each frame runs at once with the local player's input and a *predicted*
//! remote input (the last one received). When the real remote input for an
//! earlier frame arrives and differs from the prediction, the session
//! restores the snapshot taken before that frame and simulates forward
//! again with what it now knows.
//!
//! This works because the whole simulation is a value: a snapshot is
//! `Battle::clone()` and a restore is an assignment. Content runtimes keep
//! nothing between calls (the Luau VM is shared code, not state), so
//! restoring the engine state restores everything that affects the battle.
//!
//! Sound is the one output that can't be taken back. The session tags each
//! frame's cues with the frame they belong to and a [`CueLedger`] turns
//! (re)simulated frames into play and retract events, so re-simulation
//! never replays a cue that was already heard.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::hash::Hasher;

use crate::battle::{Battle, TickEvents};
use crate::input::PlayerTick;
use crate::sound::SoundCue;

/// FNV-1a, fed through `fmt::Write` so state can be hashed from its
/// `Debug` form without building a string.
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
}

impl std::fmt::Write for Fnv {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        Hasher::write(self, s.as_bytes());
        Ok(())
    }
}

/// A digest of everything the simulation carries from tick to tick (not
/// the sound output, which is per tick, nor the content handle, which is
/// code). Equal digests mean equal states; used to check that a
/// re-simulation reproduced a run bit for bit.
pub fn digest(b: &Battle) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    let _ = write!(
        h,
        "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        b.setup, b.stats, b.rng, b.round, b.fight, b.gauge, b.banner, b.paused, b.inputs, b.hands, b.transform_requests,
        b.turn_transforms, b.transform_seq,
    );
    let _ = write!(
        h,
        "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        b.beast_out_used, b.objects, b.actors, b.collision, b.field, b.fade, b.fadein_queue, b.damage_carry, b.custom_ui,
        b.sides, b.side_stats, b.linked,
    );
    h.finish()
}

/// Like [`digest`], but blind to how content state is represented: object
/// kinds' and actions' own state (`Vars`, `ActionVars`) is left out, so
/// battles run by different content runtimes (built-in, Rust content,
/// Luau) can be compared. Everything those kinds do to shared state
/// (objects' common fields, sprites, collision, actors, HP) is still in.
pub fn engine_digest(b: &Battle) -> u64 {
    use crate::object::{ObjectRef, Pool, SLOTS};
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    let _ = write!(
        h,
        "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        b.setup, b.stats, b.rng, b.round, b.fight, b.gauge, b.banner, b.paused, b.inputs, b.hands, b.transform_requests,
        b.turn_transforms, b.transform_seq, b.beast_out_used,
    );
    for pool in Pool::ALL {
        for slot in 0..SLOTS as u8 {
            let r = ObjectRef { pool, slot };
            let mut o = b.objects.get(r).clone();
            o.vars = crate::kinds::Vars::None;
            let _ = write!(h, "{}{o:?}{:?}", b.objects.is_allocated(r), b.objects.sprite(r));
        }
    }
    let order: Vec<ObjectRef> = b.objects.in_order().collect();
    let _ = write!(h, "{order:?}");
    for i in 0..crate::actor::SLOTS as u8 {
        let mut a = b.actors.get(crate::actor::ActorId(i)).clone();
        a.attack.action = Default::default();
        let _ = write!(h, "{a:?}");
    }
    let _ = write!(
        h,
        "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        b.collision, b.field, b.fade, b.fadein_queue, b.damage_carry, b.custom_ui, b.sides, b.side_stats, b.linked, b.round.ticks,
    );
    h.finish()
}

/// What happened to a sound cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueEvent {
    /// Play `cue`, which frame `frame` made.
    Play { frame: u32, cue: SoundCue },
    /// A re-simulation found frame `frame` doesn't make `cue` after all
    /// (it was played on a misprediction). A long sound can be stopped;
    /// a short one is usually left alone.
    Retract { frame: u32, cue: SoundCue },
}

/// Which cues have been played for which frame, so a re-simulated frame
/// plays only what is new and retracts what it no longer makes.
#[derive(Clone, Debug, Default)]
pub struct CueLedger {
    played: BTreeMap<u32, Vec<SoundCue>>,
}

impl CueLedger {
    /// Frame `frame` (re)simulated and made `cues`: the events that bring
    /// what was played in line with it.
    pub fn reconcile(&mut self, frame: u32, cues: &[SoundCue], out: &mut Vec<CueEvent>) {
        let played = self.played.entry(frame).or_default();
        let mut left = played.clone();
        for &cue in cues {
            match left.iter().position(|&c| c == cue) {
                Some(i) => {
                    left.remove(i);
                }
                None => {
                    played.push(cue);
                    out.push(CueEvent::Play { frame, cue });
                }
            }
        }
        for cue in left {
            let i = played.iter().position(|&c| c == cue).expect("left over from played");
            played.remove(i);
            out.push(CueEvent::Retract { frame, cue });
        }
    }

    /// Forget frames before `frame` (they can no longer be re-simulated).
    pub fn confirm_before(&mut self, frame: u32) {
        self.played = self.played.split_off(&frame);
    }
}

/// One frame's inputs as last simulated.
#[derive(Clone, Debug)]
struct FrameInputs {
    local: PlayerTick,
    remote: PlayerTick,
    events: TickEvents,
}

/// Counters for how much rolling back a session did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RollbackStats {
    pub rollbacks: u32,
    pub resimulated_frames: u32,
    pub snapshots: u32,
}

/// A rollback session for one side of a two-player battle.
pub struct Session {
    battle: Battle,
    local_side: usize,
    /// The next frame to simulate (frames count from 0).
    frame: u32,
    history: Vec<FrameInputs>,
    /// Remote inputs received, by frame (they arrive in order).
    remote: Vec<PlayerTick>,
    /// Snapshots taken before each frame that isn't confirmed yet.
    snapshots: VecDeque<(u32, Battle)>,
    cues: CueLedger,
    pending_cues: Vec<CueEvent>,
    /// Digests of confirmed frames' states, if recording.
    confirmed_digests: Option<Vec<u64>>,
    pub stats: RollbackStats,
}

impl Session {
    pub fn new(battle: Battle, local_side: u8) -> Session {
        Session {
            battle,
            local_side: local_side as usize,
            frame: 0,
            history: Vec::new(),
            remote: Vec::new(),
            snapshots: VecDeque::new(),
            cues: CueLedger::default(),
            pending_cues: Vec::new(),
            confirmed_digests: None,
            stats: RollbackStats::default(),
        }
    }

    /// Record the digest of every frame's state once it is final.
    pub fn record_digests(&mut self) {
        self.confirmed_digests = Some(Vec::new());
    }

    /// The battle as of the latest frame (possibly predicted).
    pub fn battle(&self) -> &Battle {
        &self.battle
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Digests of confirmed frames, in frame order.
    pub fn confirmed_digests(&self) -> &[u64] {
        self.confirmed_digests.as_deref().unwrap_or(&[])
    }

    /// Sound events since the last call.
    pub fn take_cue_events(&mut self) -> Vec<CueEvent> {
        std::mem::take(&mut self.pending_cues)
    }

    /// The remote input to use for `frame`: the real one if it arrived,
    /// else the last one received.
    fn remote_input(&self, frame: u32) -> PlayerTick {
        self.remote.get(frame as usize).or(self.remote.last()).cloned().unwrap_or_default()
    }

    fn confirmed(&self, frame: u32) -> bool {
        (frame as usize) < self.remote.len()
    }

    /// Simulate frame `self.frame` from the current state.
    fn simulate(&mut self) {
        let f = self.frame;
        let FrameInputs { local, remote, events } = self.history[f as usize].clone();
        if !self.confirmed(f) {
            self.snapshots.push_back((f, self.battle.clone()));
            self.stats.snapshots += 1;
        }
        let mut input = [PlayerTick::default(), PlayerTick::default()];
        input[self.local_side] = local;
        input[1 - self.local_side] = remote;
        self.battle.tick(&input, events);
        self.cues.reconcile(f, self.battle.sound_cues(), &mut self.pending_cues);
        if self.confirmed(f)
            && let Some(d) = &mut self.confirmed_digests
            && d.len() == f as usize
        {
            d.push(digest(&self.battle));
        }
        self.frame += 1;
    }

    /// Advance one frame with the local input (and the frame's shared
    /// events, which the link delivers in lockstep).
    pub fn advance(&mut self, local: PlayerTick, events: TickEvents) {
        let remote = self.remote_input(self.frame);
        self.history.push(FrameInputs { local, remote, events });
        self.simulate();
    }

    /// The remote input for the next frame in order arrived. If a frame
    /// already ran on a different prediction, roll back to it and
    /// re-simulate up to the present.
    pub fn receive_remote(&mut self, input: PlayerTick) {
        let f = self.remote.len() as u32;
        self.remote.push(input.clone());
        if f >= self.frame {
            return;
        }
        let mispredicted = self.history[f as usize].remote != input;
        if mispredicted {
            let now = self.frame;
            while self.snapshots.back().is_some_and(|&(g, _)| g > f) {
                self.snapshots.pop_back();
            }
            let (g, snapshot) = self.snapshots.pop_back().expect("a snapshot before every unconfirmed frame");
            debug_assert_eq!(g, f);
            self.battle = snapshot;
            self.frame = f;
            self.stats.rollbacks += 1;
            while self.frame < now {
                let h = self.frame;
                self.history[h as usize].remote = self.remote_input(h);
                self.simulate();
                self.stats.resimulated_frames += 1;
            }
        } else if let Some(d) = &mut self.confirmed_digests
            && d.len() == f as usize
        {
            // Predicted right: the state after frame f was already final,
            // but later frames have run since. Recompute its digest from
            // the snapshot of frame f + 1 (the state after f).
            let after = self.snapshots.iter().find(|&&(g, _)| g == f + 1).map(|(_, b)| digest(b));
            d.push(after.unwrap_or_else(|| digest(&self.battle)));
        }
        // Snapshots before confirmed frames are no longer needed.
        while self.snapshots.front().is_some_and(|&(g, _)| g <= f) {
            self.snapshots.pop_front();
        }
        self.cues.confirm_before(f);
    }
}

#[cfg(test)]
mod tests;
