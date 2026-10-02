//! Sound cues under rollback: each cue plays once.
//!
//! A rollback peer simulates some frames more than once: first on a
//! predicted input, then again when the real input arrives. Each run of
//! a tick reports its cues (`Battle::sound_cues_for`). [`CueTracker`]
//! takes every simulated tick's cues with the frame they belong to and
//! turns them into what a frontend should do:
//!
//! - a cue plays as soon as a tick first makes it, predicted or not
//!   (waiting for confirmation would delay every sound by the latency);
//! - when re-simulated frames make a cue again, it is recognised as the
//!   one already playing and not played again. It counts as the same cue
//!   if it was played for a frame at most `tolerance` frames away: a
//!   corrected input often moves an event by a frame or two;
//! - a played cue that the re-simulation no longer makes (within the
//!   tolerance) is cancelled: the frontend stops it if it is still
//!   playing, or undoes it (a music change).
//!
//! So a confirmed cue is played exactly once, and a predicted cue that
//! did not happen is played and then cancelled. Frames are the netplay
//! layer's frame numbers. Each play gets an identity ([`CueId`]), and a
//! cancel names the play it takes back
//! ([`CueTracker::drain_identified`]).

use crate::sound::SoundCue;

/// What the frontend should do with a cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CueAction {
    /// Start it now.
    Play(SoundCue),
    /// It was played on a prediction that turned out wrong: stop or undo it.
    Cancel(SoundCue),
}

/// Which play a cue action concerns: a tracker numbers its plays from 0,
/// and a cancel carries the number of the play it takes back.
pub type CueId = u32;

/// A cue handed to the frontend for a frame that isn't confirmed yet.
#[derive(Clone, Copy, Debug)]
struct Played {
    id: CueId,
    frame: u32,
    cue: SoundCue,
    /// The current simulation made it (false after a rollback past it,
    /// until a re-simulated tick makes it again).
    current: bool,
}

/// Plays each tick's cues once under rollback (see the module docs).
#[derive(Clone, Debug)]
pub struct CueTracker {
    tolerance: u32,
    played: Vec<Played>,
    actions: Vec<(CueId, CueAction)>,
    next_id: CueId,
}

impl CueTracker {
    /// A re-simulated cue up to `tolerance` frames from where it was
    /// played counts as the same cue.
    pub fn new(tolerance: u32) -> CueTracker {
        CueTracker { tolerance, played: Vec::new(), actions: Vec::new(), next_id: 0 }
    }

    /// Frames from `frame` on are about to be simulated again.
    pub fn rolled_back(&mut self, frame: u32) {
        for p in &mut self.played {
            if p.frame >= frame {
                p.current = false;
            }
        }
    }

    /// The tick for `frame` ran (for the first time or again) and made
    /// `cues`.
    pub fn simulated(&mut self, frame: u32, cues: &[SoundCue]) {
        for &cue in cues {
            let tolerance = self.tolerance;
            let same = self
                .played
                .iter_mut()
                .filter(|p| !p.current && p.cue == cue && p.frame.abs_diff(frame) <= tolerance)
                .min_by_key(|p| (p.frame.abs_diff(frame), p.frame));
            match same {
                Some(p) => {
                    p.current = true;
                    p.frame = frame;
                }
                None => {
                    let id = self.next_id;
                    self.next_id = id.wrapping_add(1);
                    self.played.push(Played { id, frame, cue, current: true });
                    self.actions.push((id, CueAction::Play(cue)));
                }
            }
        }
        // Cues the simulation has passed without making them again.
        let (tolerance, actions) = (self.tolerance, &mut self.actions);
        self.played.retain(|p| {
            let gone = !p.current && p.frame + tolerance < frame;
            if gone {
                actions.push((p.id, CueAction::Cancel(p.cue)));
            }
            !gone
        });
    }

    /// Frames before `frame` are confirmed (never simulated again).
    pub fn confirmed(&mut self, frame: u32) {
        self.played.retain(|p| !(p.current && p.frame < frame));
    }

    /// What to do, in order, since the last call.
    pub fn drain(&mut self) -> impl Iterator<Item = CueAction> + '_ {
        self.actions.drain(..).map(|(_, a)| a)
    }

    /// [`drain`](Self::drain), each action with the play it concerns: a
    /// play's own identity, or the one a cancel takes back.
    pub fn drain_identified(&mut self) -> std::vec::Drain<'_, (CueId, CueAction)> {
        self.actions.drain(..)
    }

    /// Cues played for frames not confirmed yet.
    pub fn unconfirmed(&self) -> usize {
        self.played.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::SoundId;

    const HIT: SoundCue = SoundCue::Effect(SoundId(0x6D));
    const SHOT: SoundCue = SoundCue::Effect(SoundId(0x77));

    fn run(t: &mut CueTracker, frames: std::ops::Range<u32>, cues: impl Fn(u32) -> Vec<SoundCue>) -> Vec<CueAction> {
        for f in frames {
            t.simulated(f, &cues(f));
        }
        t.drain().collect()
    }

    #[test]
    fn cues_play_when_first_simulated() {
        let mut t = CueTracker::new(2);
        let got = run(&mut t, 0..10, |f| if f % 4 == 1 { vec![SHOT] } else { vec![] });
        assert_eq!(got, [CueAction::Play(SHOT); 3]);
    }

    #[test]
    fn a_resimulation_that_makes_the_same_cues_plays_nothing() {
        let mut t = CueTracker::new(2);
        let cues = |f| if f == 5 || f == 7 { vec![HIT, SHOT] } else { vec![] };
        assert_eq!(run(&mut t, 0..10, cues).len(), 4);
        t.rolled_back(3);
        assert_eq!(run(&mut t, 3..12, cues), []);
        // Moved by a frame: still the same cue.
        t.rolled_back(4);
        assert_eq!(run(&mut t, 4..14, |f| if f == 6 || f == 7 { vec![HIT, SHOT] } else { vec![] }), []);
    }

    #[test]
    fn a_cue_the_resimulation_drops_is_cancelled_and_a_new_one_played() {
        let mut t = CueTracker::new(2);
        run(&mut t, 0..10, |f| if f == 8 { vec![HIT] } else { vec![] });
        t.rolled_back(6);
        // Up to frame 10 the hit could still turn up (tolerance 2).
        assert_eq!(run(&mut t, 6..11, |_| vec![]), []);
        assert_eq!(run(&mut t, 11..12, |_| vec![]), [CueAction::Cancel(HIT)]);
        t.rolled_back(9);
        assert_eq!(run(&mut t, 9..15, |f| if f == 9 { vec![SHOT] } else { vec![] }), [CueAction::Play(SHOT)]);
    }

    #[test]
    fn repeated_cues_are_counted() {
        let mut t = CueTracker::new(1);
        assert_eq!(run(&mut t, 0..3, |f| if f == 1 { vec![HIT, HIT] } else { vec![] }).len(), 2);
        t.rolled_back(0);
        assert_eq!(run(&mut t, 0..4, |f| if f == 1 { vec![HIT] } else { vec![] }), [CueAction::Cancel(HIT)]);
    }

    #[test]
    fn a_cancel_names_the_play_it_takes_back() {
        let mut t = CueTracker::new(1);
        for f in 0..6 {
            t.simulated(f, &if f == 1 || f == 4 { vec![HIT] } else { vec![] });
        }
        let plays: Vec<(CueId, CueAction)> = t.drain_identified().collect();
        assert_eq!(plays, [(0, CueAction::Play(HIT)), (1, CueAction::Play(HIT))]);
        // The re-simulation moves the second hit beyond the tolerance and
        // drops the first: a new play, then the first taken back, then the
        // second.
        t.rolled_back(0);
        for f in 0..8 {
            t.simulated(f, &if f == 7 { vec![HIT] } else { vec![] });
        }
        let got: Vec<(CueId, CueAction)> = t.drain_identified().collect();
        assert_eq!(got, [(0, CueAction::Cancel(HIT)), (1, CueAction::Cancel(HIT)), (2, CueAction::Play(HIT))]);
    }

    #[test]
    fn confirmed_cues_are_forgotten() {
        let mut t = CueTracker::new(2);
        run(&mut t, 0..10, |f| if f == 2 { vec![HIT] } else { vec![] });
        t.confirmed(5);
        assert_eq!(t.unconfirmed(), 0);
    }
}
