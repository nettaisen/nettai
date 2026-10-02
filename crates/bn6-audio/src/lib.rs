//! Battle audio: plays bn6-battle's sound cues with the game's own sound
//! driver (the `m4a` crate) and sound data (a content pack's sound, which
//! bn6-content loads into an `m4a::SoundBank`; `bn6-extract content` writes
//! the pack from the user's ROM).
//!
//! - [`SoundCalls`]: what the game's sound functions ask of the driver for
//!   each cue (BN6's wrappers: `PlayMusic`'s current-music check, the pinch
//!   effect's pitch and tempo, ...), as [`Request`]s.
//! - [`BattleAudio`]: cues in, samples out, a frame at a time, with the
//!   game's timing (calls queue up and run on the next frame).
//! - [`AudioOut`] (feature `playback`): the same on the default output device.
//! - [`wav`]: writing rendered audio.
//!
//! A frontend drives it once per battle tick:
//!
//! ```ignore
//! battle.tick(&input, events);
//! audio.handle(battle.sound_cues());
//! audio.tick();
//! ```
//!
//! Under rollback netplay a frontend feeds each simulated tick's cues to a
//! [`bn6_battle::cues::CueTracker`] instead and hands its actions to
//! [`BattleAudio::handle_actions`]: confirmed cues play once, and cues
//! played on a wrong prediction are stopped or undone.

use m4a::{Driver, PlayerId, SongId, SoundBank};
use std::sync::Arc;

pub use bn6_battle::cues::CueAction;
pub use bn6_battle::sound::{SoundCue, SoundId};

/// The game's "no music" song: `PlayMusic` of it stops the music (its
/// battle settings name it for a battle without music).
pub const NO_MUSIC: SoundId = SoundId(0x63);
pub use m4a;

#[cfg(feature = "playback")]
mod output;
pub mod wav;

#[cfg(feature = "playback")]
pub use output::{AudioOut, OutputError};

/// The music player BN6's background music plays on.
pub const MUSIC_PLAYER: PlayerId = PlayerId(31);
/// The player the custom screen balances against the music.
pub const CUSTOM_SCREEN_PLAYER: PlayerId = PlayerId(22);
/// Sound calls the game queues per frame (`sound_8000808` drops the rest).
pub const QUEUE_LIMIT: usize = 32;
/// Output rate, Hz.
pub const SAMPLE_RATE: u32 = m4a::OUT_RATE;
/// Frames (battle ticks) per second.
pub const FPS: f64 = m4a::FPS;

/// A call into the sound driver, as the game queues it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Request {
    /// `m4aSongNumStart`.
    Start(SongId),
    /// `m4aSongNumStop`: stop a song if its player is still playing it
    /// (only to take back a cue played on a wrong prediction).
    Stop(SongId),
    /// `m4aMPlayAllStop`.
    StopAll,
    /// `m4aMPlayTempoControl` (0x100 = as written).
    Tempo { player: PlayerId, tempo: u16 },
    /// `m4aMPlayPitchControl` (1/256 semitone) on the tracks of a bit mask.
    Pitch { player: PlayerId, tracks: u16, pitch: i16 },
    /// `m4aMPlayVolumeControl` (0x100 = full) on the tracks of a bit mask.
    Volume { player: PlayerId, tracks: u16, volume: u16 },
}

impl Request {
    /// Run it on a driver.
    pub fn apply(self, driver: &mut Driver) {
        match self {
            Request::Start(id) => {
                driver.start(id);
            }
            Request::Stop(id) => driver.stop_song(id),
            Request::StopAll => driver.stop_all(),
            Request::Tempo { player, tempo } => driver.set_tempo(player, tempo),
            Request::Pitch { player, tracks, pitch } => driver.set_pitch(player, tracks, pitch),
            Request::Volume { player, tracks, volume } => driver.set_volume(player, tracks, volume),
        }
    }
}

/// BN6's sound functions: each cue as the driver calls the game queues.
#[derive(Clone, Debug)]
pub struct SoundCalls {
    /// The music `PlayMusic` last started (GameState's BGMusicIndicator;
    /// 0xFF: none).
    music: u8,
    /// What it was before the last change (to undo a cancelled change).
    previous_music: u8,
}

impl Default for SoundCalls {
    fn default() -> SoundCalls {
        SoundCalls { music: NO_INDICATOR, previous_music: NO_INDICATOR }
    }
}

const NO_INDICATOR: u8 = 0xFF;

impl SoundCalls {
    pub fn new() -> SoundCalls {
        SoundCalls::default()
    }

    /// The driver calls for a cue, appended to `out`.
    pub fn requests(&mut self, cue: SoundCue, out: &mut Vec<Request>) {
        match cue {
            // PlaySoundEffect: m4aSongNumStart through the queue.
            SoundCue::Effect(id) => out.push(Request::Start(SongId(id.0))),
            // PlayMusic: nothing if it's the music already.
            SoundCue::Music(id) => {
                if id.0 == self.music as u16 {
                    return;
                }
                self.previous_music = self.music;
                self.music = id.0 as u8;
                out.push(if id == NO_MUSIC { Request::StopAll } else { Request::Start(SongId(id.0)) });
            }
            // musicGameState_8000784.
            SoundCue::StopMusic => {
                self.previous_music = self.music;
                self.music = NO_INDICATOR;
                out.push(Request::StopAll);
            }
            // sub_8009158: a semitone up at 0x11A/0x100 tempo, or back.
            SoundCue::Pinch(on) => {
                let (pitch, tempo) = if on { (0x100, 0x11A) } else { (0, 0x100) };
                out.push(Request::Pitch { player: MUSIC_PLAYER, tracks: 0xFFFF, pitch });
                out.push(Request::Tempo { player: MUSIC_PLAYER, tempo });
            }
            // sub_802A3CC.
            SoundCue::RestoreVolume => {
                for player in [MUSIC_PLAYER, CUSTOM_SCREEN_PLAYER] {
                    out.push(Request::Volume { player, tracks: 0xFFFF, volume: 0x100 });
                }
            }
            // sub_802A30C, sub_802A362.
            SoundCue::ScreenVolume { music, screen } => {
                out.push(Request::Volume { player: MUSIC_PLAYER, tracks: 0xFFFF, volume: music });
                out.push(Request::Volume { player: CUSTOM_SCREEN_PLAYER, tracks: 0xFFFF, volume: screen });
            }
        }
    }
}

impl SoundCalls {
    /// The driver calls that take back a cue played on a wrong prediction
    /// (rollback netplay), appended to `out`: a sound effect stops if it
    /// is still playing, a music change goes back to the music before it
    /// (from its start), the pinch effect is switched back.
    pub fn cancel(&mut self, cue: SoundCue, out: &mut Vec<Request>) {
        match cue {
            SoundCue::Effect(id) => out.push(Request::Stop(SongId(id.0))),
            SoundCue::Music(id) if id.0 == self.music as u16 => self.restore_previous_music(out),
            SoundCue::StopMusic if self.music == NO_INDICATOR => self.restore_previous_music(out),
            SoundCue::Pinch(on) => self.requests(SoundCue::Pinch(!on), out),
            // A later change replaced it already; the custom screen's
            // volume can't be taken back.
            SoundCue::Music(_) | SoundCue::StopMusic | SoundCue::RestoreVolume | SoundCue::ScreenVolume { .. } => {}
        }
    }

    fn restore_previous_music(&mut self, out: &mut Vec<Request>) {
        let music = std::mem::replace(&mut self.music, self.previous_music);
        self.previous_music = music;
        out.push(match self.music {
            NO_INDICATOR => Request::StopAll,
            m if m as u16 == NO_MUSIC.0 => Request::StopAll,
            m => Request::Start(SongId(m as u16)),
        });
    }
}

/// The game's sound, fed with cues and rendered a frame at a time.
pub struct BattleAudio {
    driver: Driver,
    calls: SoundCalls,
    queue: Vec<Request>,
}

impl BattleAudio {
    pub fn new(bank: Arc<SoundBank>) -> BattleAudio {
        BattleAudio { driver: Driver::new(bank), calls: SoundCalls::new(), queue: Vec::new() }
    }

    /// Queue a tick's cues; they run at the start of the next frame, as
    /// the game's queued sound calls do.
    pub fn handle(&mut self, cues: &[SoundCue]) {
        let mut requests = Vec::new();
        for &cue in cues {
            self.calls.requests(cue, &mut requests);
        }
        for r in requests {
            if self.queue.len() < QUEUE_LIMIT {
                self.queue.push(r);
            }
        }
    }

    /// Queue rollback cue actions (see [`bn6_battle::cues`]): plays as
    /// [`handle`](Self::handle) does, and cancels of cues played on a
    /// wrong prediction.
    pub fn handle_actions(&mut self, actions: impl IntoIterator<Item = CueAction>) {
        let mut requests = Vec::new();
        for action in actions {
            match action {
                CueAction::Play(cue) => self.calls.requests(cue, &mut requests),
                CueAction::Cancel(cue) => self.calls.cancel(cue, &mut requests),
            }
        }
        for r in requests {
            if self.queue.len() < QUEUE_LIMIT {
                self.queue.push(r);
            }
        }
    }

    /// One frame: the driver's VBlank (sequencers and mix), then the
    /// queued calls (the game's main loop runs them next). Appends the
    /// frame's samples (about 549 at 32768 Hz, stereo, +-1.0) to `out`.
    pub fn tick(&mut self, out: &mut Vec<[f32; 2]>) {
        self.driver.step_frame();
        for r in std::mem::take(&mut self.queue) {
            r.apply(&mut self.driver);
        }
        self.driver.take_output(out);
    }

    pub fn driver(&self) -> &Driver {
        &self.driver
    }

    pub fn driver_mut(&mut self) -> &mut Driver {
        &mut self.driver
    }
}

#[cfg(test)]
mod tests;
