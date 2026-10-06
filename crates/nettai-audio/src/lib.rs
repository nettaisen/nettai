//! Battle audio: plays nettai-battle's sound cues with the game's own sound
//! driver (the `m4a` crate) and sound data (a content pack's sound, which
//! nettai-content loads into an `m4a::SoundBank`; `nettai-extract exe6` writes
//! the pack from the user's ROM).
//!
//! - [`Songs`]: what each of the engine's sounds (a handle over the loaded
//!   packs' sounds) is in its pack's song table; [`Songs::cue`] turns a
//!   cue's sounds into songs.
//! - [`SoundCalls`]: what the game's sound functions ask of the driver for
//!   each cue in songs (EXE6's wrappers: `PlayMusic`'s current-music check,
//!   the pinch effect's pitch and tempo, ...), as [`Request`]s.
//! - [`BattleAudio`]: cues in, samples out, a frame at a time, with the
//!   game's timing (calls queue up and run on the next frame).
//! - [`AudioOut`] (feature `playback`): the same on the default output device;
//!   [`Output`] is that device alone, playing samples it is given.
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
//! [`nettai_battle::cues::CueTracker`] instead and hands its actions to
//! [`BattleAudio::handle_actions`]: confirmed cues play once, and cues
//! played on a wrong prediction are stopped or undone.

use m4a::{Driver, PlayerId, SongId, SoundBank};
use std::sync::Arc;

pub use nettai_battle::cues::CueAction;
pub use nettai_battle::sound::{SoundCue, SoundId};

/// The game's "no music" song: `PlayMusic` of it stops the music (its
/// battle settings name it for a battle without music). A song, not a
/// sound handle: a cue's sounds are songs once [`Songs::cue`] has them.
pub const NO_MUSIC: SoundId = SoundId(0x63);

/// What the engine's sounds are in their packs' song tables (docs/design/
/// rules-in-luau.md §7.4: the engine knows a sound by its handle), by
/// handle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Songs(pub Vec<nettai_battle::content::InPack<u16>>);

impl Songs {
    /// The loaded packs' sounds' songs.
    pub fn of(assets: &nettai_battle::content::AssetNames) -> Songs {
        Songs(assets.sounds.values().copied().collect())
    }

    /// Songs by their own numbers (sound `n` is song `n` of the one pack):
    /// for tools that play a bank's songs, not the engine's sounds.
    pub fn numbers(count: usize) -> Songs {
        Songs((0..count).map(|n| nettai_battle::content::InPack { pack: Default::default(), id: n as u16 }).collect())
    }

    /// Sound `id`'s song.
    pub fn song(&self, id: SoundId) -> SoundId {
        SoundId(self.0.get(id.0 as usize).unwrap_or_else(|| panic!("no sound has handle {}", id.0)).id)
    }

    /// `cue` with its sounds as songs: what [`SoundCalls`] takes.
    pub fn cue(&self, cue: SoundCue) -> SoundCue {
        match cue {
            SoundCue::Effect(id) => SoundCue::Effect(self.song(id)),
            SoundCue::Music(id) => SoundCue::Music(self.song(id)),
            other => other,
        }
    }
}
pub use m4a;

#[cfg(feature = "playback")]
mod output;
pub mod wav;

#[cfg(feature = "playback")]
pub use output::{AudioOut, Output, OutputError};

/// The music player EXE6's background music plays on.
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

/// EXE6's sound functions: each cue as the driver calls the game queues.
#[derive(Clone, Debug)]
pub struct SoundCalls {
    /// The music `PlayMusic` last started (GameState's BGMusicIndicator;
    /// 0xFF: none).
    music: u8,
    /// What it was before the last change (to undo a canceled change).
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

    /// The driver calls for a cue (its sounds as songs: [`Songs::cue`]),
    /// appended to `out`.
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

/// The game's sound, fed with cues and rendered a frame at a time: a
/// driver for each loaded pack's sound (docs/design/rules-in-luau.md §7.4:
/// a song plays with its own pack's instruments; two packs' banks never
/// mix inside one player), their outputs added.
pub struct BattleAudio {
    packs: Vec<PackSound>,
    songs: Songs,
    /// The pack whose music player plays the battle's music, once some
    /// music has started.
    music: Option<usize>,
    mix: Vec<[f32; 2]>,
}

/// One pack's driver and the calls queued for it.
struct PackSound {
    driver: Driver,
    calls: SoundCalls,
    queue: Vec<Request>,
}

impl BattleAudio {
    /// The sound of `bank`, playing the engine's sounds as `songs` says
    /// (one pack's).
    pub fn new(bank: Arc<SoundBank>, songs: Songs) -> BattleAudio {
        BattleAudio::with_banks(vec![bank], songs)
    }

    /// The sound of several packs, `banks` by `PackId`.
    pub fn with_banks(banks: Vec<Arc<SoundBank>>, songs: Songs) -> BattleAudio {
        assert!(!banks.is_empty(), "a pack's sound");
        let packs = banks.into_iter().map(|b| PackSound { driver: Driver::new(b), calls: SoundCalls::new(), queue: Vec::new() }).collect();
        BattleAudio { packs, songs, music: None, mix: Vec::new() }
    }

    /// The pack a cue plays on, and the cue in its songs: a sound's own
    /// pack; what changes the music, the music's pack (the first's before
    /// any).
    fn route(&self, cue: SoundCue) -> (usize, SoundCue) {
        let pack_of = |id: SoundId| {
            let a = self.songs.0.get(id.0 as usize).unwrap_or_else(|| panic!("no sound has handle {}", id.0));
            a.pack.index().min(self.packs.len() - 1)
        };
        match cue {
            SoundCue::Effect(id) | SoundCue::Music(id) => (pack_of(id), self.songs.cue(cue)),
            other => (self.music.unwrap_or(0), other),
        }
    }

    /// Queue `requests` for pack `pack`'s driver.
    fn queue(&mut self, pack: usize, requests: Vec<Request>) {
        let q = &mut self.packs[pack].queue;
        for r in requests {
            if q.len() < QUEUE_LIMIT {
                q.push(r);
            }
        }
    }

    /// Play `cue`: a music change on another pack's player than the music's
    /// stops that one first; stopping the music stops every pack's.
    fn play(&mut self, cue: SoundCue) {
        if let SoundCue::StopMusic = cue {
            for p in 0..self.packs.len() {
                let mut r = Vec::new();
                self.packs[p].calls.requests(SoundCue::StopMusic, &mut r);
                self.queue(p, r);
            }
            return;
        }
        let (pack, cue) = self.route(cue);
        if let SoundCue::Music(_) = cue {
            if let Some(old) = self.music.filter(|&m| m != pack) {
                let mut r = Vec::new();
                self.packs[old].calls.requests(SoundCue::StopMusic, &mut r);
                self.queue(old, r);
            }
            self.music = Some(pack);
        }
        let mut r = Vec::new();
        self.packs[pack].calls.requests(cue, &mut r);
        self.queue(pack, r);
    }

    /// Queue a tick's cues; they run at the start of the next frame, as
    /// the game's queued sound calls do.
    pub fn handle(&mut self, cues: &[SoundCue]) {
        for &cue in cues {
            self.play(cue);
        }
    }

    /// Queue rollback cue actions (see [`nettai_battle::cues`]): plays as
    /// [`handle`](Self::handle) does, and cancels of cues played on a
    /// wrong prediction.
    pub fn handle_actions(&mut self, actions: impl IntoIterator<Item = CueAction>) {
        for action in actions {
            match action {
                CueAction::Play(cue) => self.play(cue),
                CueAction::Cancel(cue) => {
                    let (pack, cue) = self.route(cue);
                    let mut r = Vec::new();
                    self.packs[pack].calls.cancel(cue, &mut r);
                    self.queue(pack, r);
                }
            }
        }
    }

    /// One frame: each driver's VBlank (sequencers and mix), then its
    /// queued calls (the game's main loop runs them next). Appends the
    /// frame's samples (about 549 at 32768 Hz, stereo, +-1.0; every
    /// pack's added) to `out`.
    pub fn tick(&mut self, out: &mut Vec<[f32; 2]>) {
        let start = out.len();
        for (i, p) in self.packs.iter_mut().enumerate() {
            p.driver.step_frame();
            for r in std::mem::take(&mut p.queue) {
                r.apply(&mut p.driver);
            }
            if i == 0 {
                p.driver.take_output(out);
            } else {
                self.mix.clear();
                p.driver.take_output(&mut self.mix);
                for (o, m) in out[start..].iter_mut().zip(&self.mix) {
                    o[0] += m[0];
                    o[1] += m[1];
                }
            }
        }
    }

    /// The first pack's driver.
    pub fn driver(&self) -> &Driver {
        &self.packs[0].driver
    }

    pub fn driver_mut(&mut self) -> &mut Driver {
        &mut self.packs[0].driver
    }

    /// Pack `pack`'s driver.
    pub fn driver_of(&self, pack: nettai_battle::content::PackId) -> Option<&Driver> {
        self.packs.get(pack.index()).map(|p| &p.driver)
    }
}

#[cfg(test)]
mod tests;
