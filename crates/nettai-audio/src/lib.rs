//! Battle audio: plays nettai-battle's sound cues with the game's own sound
//! driver (the `m4a` crate) and sound data (a content pack's sound, which
//! nettai-content loads into an `m4a::SoundBank`; `nettai-extract content/exe6` writes
//! the pack from the user's ROM).
//!
//! - [`Songs`]: what each of the engine's sounds (a handle over the loaded
//!   packs' sounds) is in its pack's song table; [`Songs::cue`] turns a
//!   cue's sounds into songs.
//! - [`Theme`] and [`with_theme`]: a battle's music in place of its game's
//!   own, a song of any game's pack played with that pack's bank.
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
    /// A content's sounds' songs in its game's pack (`assets`: what the
    /// pack holds of the content's assets), the pack the first loaded.
    pub fn of(assets: &nettai_battle::content::AssetNumbers) -> Songs {
        Songs(assets.sounds.iter().map(|&id| nettai_battle::content::InPack { pack: Default::default(), id }).collect())
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

    /// These songs with the sounds `music` (a battle's own music: its
    /// content's `battle_music`) playing song `to` in their place.
    pub fn with_music(&self, music: &[SoundId], to: nettai_battle::content::InPack<u16>) -> Songs {
        let mut songs = self.clone();
        for &m in music {
            if let Some(s) = songs.0.get_mut(m.0 as usize) {
                *s = to;
            }
        }
        songs
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

/// A song of a pack's, with that pack's bank: a battle's music in place of
/// its game's own (a battle theme a player picked, of the battle's game or
/// another's). The song plays on a driver of its own bank, so with its own
/// pack's voice groups and samples, and the battle stays its game's: its
/// effects and its result's music are its game's pack's, as before.
#[derive(Clone)]
pub struct Theme {
    pub bank: Arc<SoundBank>,
    /// Its number in the bank's song table.
    pub song: u16,
}

/// A battle's sound, its packs' `banks` (by `PackId`) and its `songs`, with
/// `theme`'s song as its music in place of its own (`music`: the sounds
/// that are its own music, `Content::battle_music`): the theme's bank one
/// pack more, unless it is one of `banks` already (a theme of the battle's
/// own game, which plays on its game's driver as the game's own music does).
pub fn with_theme(mut banks: Vec<Arc<SoundBank>>, songs: &Songs, music: &[SoundId], theme: &Theme) -> (Vec<Arc<SoundBank>>, Songs) {
    let pack = match banks.iter().position(|b| Arc::ptr_eq(b, &theme.bank)) {
        Some(p) => p,
        None => {
            banks.push(theme.bank.clone());
            banks.len() - 1
        }
    };
    let to = nettai_battle::content::InPack { pack: nettai_battle::content::PackId(pack as u8), id: theme.song };
    (banks, songs.with_music(music, to))
}

#[cfg(feature = "playback")]
mod output;
pub mod wav;

#[cfg(feature = "playback")]
pub use output::{AudioOut, MAX_RATE_SHIFT, Output, OutputError, OutputStats, RESUME_FILL, TARGET_FILL};

/// The music player EXE6's background music plays on.
pub const MUSIC_PLAYER: PlayerId = PlayerId(31);
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
    /// The music `PlayMusic` last started (GameState's BGMusicIndicator, a
    /// byte there: every game's music is a song under 0x100; held whole
    /// here, so a bank of more songs is no other; 0xFFFF: none).
    music: u16,
    /// What it was before the last change (to undo a canceled change).
    previous_music: u16,
}

impl Default for SoundCalls {
    fn default() -> SoundCalls {
        SoundCalls { music: NO_INDICATOR, previous_music: NO_INDICATOR }
    }
}

const NO_INDICATOR: u16 = 0xFFFF;

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
                if id.0 == self.music {
                    return;
                }
                self.previous_music = self.music;
                self.music = id.0;
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
            // sub_8000642: m4aMPlayVolumeControl on all of its tracks.
            SoundCue::Volume { player, volume } => out.push(Request::Volume { player: PlayerId(player), tracks: 0xFFFF, volume }),
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
            SoundCue::Music(id) if id.0 == self.music => self.restore_previous_music(out),
            SoundCue::StopMusic if self.music == NO_INDICATOR => self.restore_previous_music(out),
            SoundCue::Pinch(on) => self.requests(SoundCue::Pinch(!on), out),
            // A later change replaced it already; the custom screen's
            // volume can't be taken back.
            SoundCue::Music(_) | SoundCue::StopMusic | SoundCue::Volume { .. } => {}
        }
    }

    /// The music moved to another pack's player: this one's song stops
    /// (that alone: m4aSongNumStop, the pack's effects playing on), and is
    /// what a taken-back move starts again (`cancel` of a `StopMusic`).
    fn stop_moved(&mut self, out: &mut Vec<Request>) {
        self.previous_music = std::mem::replace(&mut self.music, NO_INDICATOR);
        if self.previous_music != NO_INDICATOR && self.previous_music != NO_MUSIC.0 {
            out.push(Request::Stop(SongId(self.previous_music)));
        }
    }

    /// Take back music `id`, which moved the music here from another pack's
    /// player (it goes back there): it stops, and the music before it here
    /// is this player's again, without starting.
    fn take_back_moved(&mut self, id: SoundId, out: &mut Vec<Request>) {
        self.music = std::mem::replace(&mut self.previous_music, self.music);
        out.push(if id == NO_MUSIC { Request::StopAll } else { Request::Stop(SongId(id.0)) });
    }

    fn restore_previous_music(&mut self, out: &mut Vec<Request>) {
        let music = std::mem::replace(&mut self.music, self.previous_music);
        self.previous_music = music;
        out.push(match self.music {
            NO_INDICATOR => Request::StopAll,
            m if m == NO_MUSIC.0 => Request::StopAll,
            m => Request::Start(SongId(m)),
        });
    }
}

/// The game's sound, fed with cues and rendered a frame at a time: a
/// driver for each loaded pack's sound (docs/design/rules-in-luau.md §7.4:
/// a song plays with its own pack's instruments; two packs' banks never
/// mix inside one player), their outputs added.
///
/// A copy of it is the sound as it stands, to come back to: fed the same
/// cues from there, it gives the same samples (the banks are shared, the
/// rest is the drivers' state). A replay viewer keeps one with each
/// snapshot of the battle, so a seek lands in the sound that playing
/// straight there makes.
#[derive(Clone)]
pub struct BattleAudio {
    packs: Vec<PackSound>,
    songs: Songs,
    /// The pack whose music player plays the battle's music, once some
    /// music has started.
    music: Option<usize>,
    /// The pack the last music change moved the music from (it stopped
    /// that pack's), to move it back there if the change is taken back.
    moved_from: Option<usize>,
    /// The battle's music as its cues name it (a sound's handle; none:
    /// stopped), and what it was before the last change: what a change of
    /// the songs ([`BattleAudio::set_songs`]) starts again as its new song.
    heard: Option<SoundId>,
    heard_before: Option<SoundId>,
    /// The low-HP music's switch is on (`Pinch`).
    pinch: bool,
    /// The host's levels ([`BattleAudio::set_levels`]), for a pack added
    /// later.
    levels: (f32, f32),
    mix: Vec<[f32; 2]>,
}

/// `driver` at the host's levels, the music's and the effects'
/// ([`BattleAudio::set_levels`]).
fn leveled(driver: &mut Driver, (music, effects): (f32, f32)) {
    let level = |l: f32| (l.clamp(0.0, 1.0) * 256.0).round() as u16;
    for player in 0..driver.players().len() {
        let id = PlayerId(player as u8);
        driver.set_level(id, level(if id == MUSIC_PLAYER { music } else { effects }));
    }
}

/// One pack's driver and the calls queued for it.
#[derive(Clone)]
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
        BattleAudio { packs, songs, music: None, moved_from: None, heard: None, heard_before: None, pinch: false, levels: (1.0, 1.0), mix: Vec::new() }
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
    /// stops that one first; stopping the music stops every pack's; a
    /// player's volume is set in every pack (as one driver's would be: the
    /// music's pack has the music player's to set, the effects' the others').
    fn play(&mut self, cue: SoundCue) {
        match cue {
            SoundCue::StopMusic => self.heard_before = std::mem::take(&mut self.heard),
            SoundCue::Pinch(on) => self.pinch = on,
            _ => {}
        }
        if let SoundCue::StopMusic | SoundCue::Volume { .. } = cue {
            for p in 0..self.packs.len() {
                let mut r = Vec::new();
                self.packs[p].calls.requests(cue, &mut r);
                self.queue(p, r);
            }
            return;
        }
        let handle = cue;
        let (pack, cue) = self.route(cue);
        if let (SoundCue::Music(id), SoundCue::Music(handle)) = (cue, handle) {
            // (PlayMusic of the music already playing changes nothing.)
            if id.0 != self.packs[pack].calls.music {
                self.heard_before = self.heard.replace(handle);
                // Another pack's music player stops its song (that alone: the
                // pack's effects play on).
                self.moved_from = self.music.filter(|&m| m != pack);
                if let Some(old) = self.moved_from {
                    let mut r = Vec::new();
                    self.packs[old].calls.stop_moved(&mut r);
                    self.queue(old, r);
                }
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
                    // (What the music was, as the cues name it, is again what it
                    // was where the change is taken back; the low-HP switch, the
                    // other way.)
                    let taken_back = match cue {
                        SoundCue::Music(id) => self.packs[pack].calls.music == id.0,
                        SoundCue::StopMusic => self.packs[pack].calls.music == NO_INDICATOR,
                        _ => false,
                    };
                    if taken_back {
                        std::mem::swap(&mut self.heard, &mut self.heard_before);
                    }
                    if let SoundCue::Pinch(on) = cue {
                        self.pinch = !on;
                    }
                    // A music change that moved the music here from another
                    // pack's player moves it back: this pack's song stops,
                    // and the other's starts again from its start (as a
                    // change taken back on one player does).
                    if let (SoundCue::Music(id), Some(from)) = (cue, self.moved_from)
                        && self.music == Some(pack)
                        && self.packs[pack].calls.music == id.0
                    {
                        self.packs[pack].calls.take_back_moved(id, &mut r);
                        self.queue(pack, r);
                        let mut back = Vec::new();
                        self.packs[from].calls.cancel(SoundCue::StopMusic, &mut back);
                        self.queue(from, back);
                        self.music = Some(from);
                        self.moved_from = None;
                    } else {
                        self.packs[pack].calls.cancel(cue, &mut r);
                        self.queue(pack, r);
                    }
                }
            }
        }
    }

    /// The host's levels, each 0 to 1 (1: as the game plays it): the
    /// music's, which is the music player's notes, and the effects', which
    /// is every other player's ([`m4a::Driver::set_level`]: what sounds is
    /// the same, only how loud each is mixed). A host's "Music" and "Sound
    /// effects". They are the drivers' state: a copy kept before a change
    /// has the levels it was kept with.
    pub fn set_levels(&mut self, music: f32, effects: f32) {
        self.levels = (music, effects);
        for p in &mut self.packs {
            leveled(&mut p.driver, self.levels);
        }
    }

    /// Play the battle's sounds as `songs` says from now on, with `banks`
    /// (by the `PackId`s `songs` names): a host's change of the battle's
    /// music while it runs (a battle theme picked, or the game's own again:
    /// [`with_theme`]'s parts, or the game's own parts). A bank playing
    /// already is that pack (found by its bank); another is one pack more, at
    /// the host's levels. Where the battle's music playing now is another
    /// song by them, that one starts in its place, from its beginning, on its
    /// own pack's player (the song before stopped, and the low-HP switch kept
    /// on where it is); the result's music, or music stopped, stays as it is,
    /// and the next round's start plays the new song.
    pub fn set_songs(&mut self, banks: Vec<Arc<SoundBank>>, songs: Songs) {
        let packs: Vec<usize> = banks.iter().map(|bank| self.pack_of(bank)).collect();
        let place = |s: nettai_battle::content::InPack<u16>| nettai_battle::content::InPack {
            pack: nettai_battle::content::PackId(packs.get(s.pack.index()).copied().unwrap_or(0) as u8),
            id: s.id,
        };
        let songs = Songs(songs.0.into_iter().map(place).collect());
        let playing = self.heard.map(|h| (h, self.songs.0.get(h.0 as usize).copied()));
        self.songs = songs;
        if let Some((h, was)) = playing
            && self.songs.0.get(h.0 as usize).copied() != was
        {
            self.play(SoundCue::Music(h));
            if self.pinch {
                self.play(SoundCue::Pinch(true));
            }
        }
    }

    /// The pack whose bank is `bank`: one playing already, else one more,
    /// at the host's levels.
    fn pack_of(&mut self, bank: &Arc<SoundBank>) -> usize {
        if let Some(p) = self.packs.iter().position(|p| Arc::ptr_eq(p.driver.bank(), bank)) {
            return p;
        }
        let mut driver = Driver::new(bank.clone());
        leveled(&mut driver, self.levels);
        self.packs.push(PackSound { driver, calls: SoundCalls::new(), queue: Vec::new() });
        self.packs.len() - 1
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
