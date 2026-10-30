//! The sound: the audio device, the battle's samples to it, the menus' sounds and the title music.
//!
//! The menus' sounds are the selected game's own: its custom screen's cursor, pick and taking back, its
//! refusal, and the sound its battles start with as the screen breaks into one (the jack-in's), as its rules'
//! sound roles name them (the app names no sound of a game's graphics). The title music is the
//! game's too: the song its assets' index names `title-screen` (no sound of the content's: played as one more past the
//! content's sounds), played across the menu screens, faded out as a battle starts, and crossfaded with the next
//! game's as the game changes (docs/design/app-ui.md §6).
//!
//! The device gets one stream at the battle's rate. While a battle runs, the menus' sounds are ticked with
//! its ticks and added to its samples; with none running they run on their own clock at the same rate
//! (docs/app.md §1, "The sound").

use nettai_audio::{BattleAudio, Output, SoundCue, SoundId};
use nettai_battle::content::SoundRole;
use nettai_frontend::game::Loaded;
use nettai_frontend::player::FRAME_RATE;
use std::time::Duration;
use nettai_platform::time::Instant;

/// The name a game's assets' index gives its title screen's music.
const TITLE_MUSIC: &str = "title-screen";

/// How long the title music takes to fade out, and to fade in.
const FADE: f32 = 0.9;
const FADE_IN: f32 = 0.45;

/// The device's level at full volume (the GBA's full scale is loud).
const FULL: f32 = 0.8;

/// A menu sound, by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// The cursor moved.
    Cursor,
    /// Something was chosen.
    Pick,
    /// Back.
    Back,
    /// It can't be done.
    Refused,
    /// A battle is entered (the jack-in): the game's own encounter sound, where its content names one.
    JackIn,
}

/// The selected game's sound for the menus: its effects (the cues of its roles) and its title music, each on a
/// driver of its own so their levels are set apart.
struct Menu {
    effects: BattleAudio,
    cursor: Option<SoundCue>,
    pick: Option<SoundCue>,
    back: Option<SoundCue>,
    refused: Option<SoundCue>,
    jack_in: Option<SoundCue>,
    music: BattleAudio,
    title: Option<SoundCue>,
    /// The title music is playing (or fading out).
    playing: bool,
}

pub struct Sound {
    out: Option<Output>,
    menu: Option<Menu>,
    /// Which game's sounds the menus have.
    game: Option<String>,
    /// The levels, 0 to 1: the music's, and the effects' (the battle's sound is at the effects').
    music: f32,
    effects: f32,
    /// The title music is wanted (a menu screen shows), and how much of it there is as it fades (0 to 1).
    wanted: bool,
    fade: f32,
    /// The game before's title music, fading out under this one's, and how much of it is left.
    leaving: Option<(BattleAudio, f32)>,
    faded: Vec<[f32; 2]>,
    last: Option<Instant>,
    owed: f64,
    mix: Vec<[f32; 2]>,
    tune: Vec<[f32; 2]>,
    /// `NETTAI_PLAY_STATS`: since when, and the loudest sample of the title music and of the whole stream since.
    stats: Option<(Instant, f32, f32)>,
}

impl Sound {
    pub fn open() -> Sound {
        let out = match Output::open() {
            Ok(o) => {
                o.set_volume(FULL);
                Some(o)
            }
            Err(e) => {
                crate::errors::log("opening the sound device", &e);
                None
            }
        };
        Sound { out, stats: std::env::var_os("NETTAI_PLAY_STATS").map(|_| (Instant::now(), 0.0, 0.0)), ..Sound::silent() }
    }

    /// The player clicked or pressed a key: a browser starts a page's sound only then, so the device is started
    /// again (where it runs already, nothing changes).
    #[cfg(target_arch = "wasm32")]
    pub fn resume(&self) {
        if let Some(out) = &self.out {
            out.resume();
        }
    }

    /// No device: nothing is heard (the screenshot pass).
    pub fn silent() -> Sound {
        Sound { out: None, menu: None, game: None, music: 0.8, effects: 1.0, wanted: false, fade: 0.0, leaving: None, faded: Vec::new(), last: None, owed: 0.0, mix: Vec::new(), tune: Vec::new(), stats: None }
    }

    /// The sound queued for the device, in seconds.
    pub fn queued(&self) -> f64 {
        self.out.as_ref().map_or(0.0, Output::buffered)
    }

    /// The levels, each 0 to 100.
    pub fn set_levels(&mut self, music: u32, effects: u32) {
        self.music = music.min(100) as f32 / 100.0;
        self.effects = effects.min(100) as f32 / 100.0;
    }

    /// The levels a battle's sound is to be given (`Player::set_sound_levels`, a viewer's): its music's and
    /// its effects', against the louder of the two, which is what its samples are played at here. (The game's
    /// driver mixes the two with eight bits: turning one down inside it is coarser than turning samples down,
    /// so only the difference between the two is the driver's.)
    pub fn battle_levels(&self) -> (f32, f32) {
        let most = self.music.max(self.effects);
        if most <= 0.0 { (0.0, 0.0) } else { (self.music / most, self.effects / most) }
    }

    /// The menus' sounds and music from `game` (its id), if they aren't its already.
    pub fn use_game(&mut self, id: &str, game: &Loaded) {
        if self.game.as_deref() == Some(id) {
            return;
        }
        let Some(sound) = &game.sound else { return };
        let content = &game.game.content;
        let roles = &content.defs.roles().sounds;
        let cue = |role| roles.get(&role).map(|&id| SoundCue::Effect(id));
        // The music of the game before fades out under this one's.
        if let Some(old) = self.menu.take().filter(|m| m.playing) {
            self.leaving = Some((old.music, self.fade));
        }
        self.fade = 0.0;
        // (The title music is no sound of the content's: the game's song, one more sound past the content's.)
        let mut music = sound.songs.clone();
        let title = game.game.index.sounds.get(TITLE_MUSIC).map(|&id| {
            music.0.push(nettai_audio::BankSong { bank: Default::default(), song: id });
            SoundCue::Music(SoundId(music.0.len() as u16 - 1))
        });
        self.menu = Some(Menu {
            effects: BattleAudio::new(sound.bank.clone(), sound.songs.clone()),
            cursor: cue(SoundRole::CustomCursor),
            pick: cue(SoundRole::CustomPick),
            back: cue(SoundRole::CustomBack),
            refused: cue(SoundRole::Refused),
            jack_in: cue(SoundRole::Encounter),
            music: BattleAudio::new(sound.bank.clone(), music),
            title,
            playing: false,
        });
        self.game = Some(id.to_string());
    }

    pub fn play(&mut self, cue: Cue) {
        let Some(m) = &mut self.menu else { return };
        let cue = match cue {
            Cue::Cursor => m.cursor,
            Cue::Pick => m.pick,
            Cue::Back => m.back,
            Cue::Refused => m.refused,
            Cue::JackIn => m.jack_in,
        };
        if let Some(cue) = cue {
            m.effects.handle(&[cue]);
        }
    }

    /// Whether the title music should play: on the menu screens, not in a battle or on the cards before one.
    /// It starts from its beginning, and stops by fading out.
    pub fn want_music(&mut self, wanted: bool) {
        self.wanted = wanted;
    }

    /// A frame at `now`, which ran `ticks` of a battle that made `battle`'s samples (none: no battle runs, or
    /// it is paused): the menus' sounds and the title music are ticked with the battle's ticks, their samples
    /// added to the battle's where they fall; with no battle running, on their own clock at the same rate.
    /// What there is goes to the device.
    pub fn frame(&mut self, now: Instant, ticks: Option<u32>, battle: &mut Vec<[f32; 2]>) {
        // (A phone's app back from the background, or its sound given back after a call: the device stopped then,
        // and is started again.)
        if nettai_platform::lifecycle::resumed()
            && let Some(out) = &self.out
        {
            out.resume();
        }
        let elapsed = self.last.map_or(Duration::ZERO, |l| now.saturating_duration_since(l)).min(Duration::from_millis(250));
        self.last = Some(now);
        let due = match ticks {
            Some(n) => {
                self.owed = 0.0;
                n
            }
            None => {
                self.owed += elapsed.as_secs_f64() * FRAME_RATE;
                let due = self.owed.floor();
                self.owed -= due;
                due as u32
            }
        };
        self.mix.clear();
        self.tune.clear();
        self.faded.clear();
        let mut fade = (self.fade, self.fade);
        if let Some(m) = &mut self.menu {
            // The title music: started (from nothing, fading in) when it is wanted, faded out and then stopped
            // when it isn't.
            let (music, cue, playing) = (&mut m.music, m.title, &mut m.playing);
            if self.wanted && !*playing {
                if let Some(cue) = cue {
                    music.handle(&[cue]);
                    *playing = true;
                }
            }
            if self.wanted {
                self.fade = (self.fade + elapsed.as_secs_f32() / FADE_IN).min(1.0);
            } else if *playing {
                self.fade = (self.fade - elapsed.as_secs_f32() / FADE).max(0.0);
                if self.fade == 0.0 {
                    music.handle(&[SoundCue::StopMusic]);
                    *playing = false;
                }
            }
            fade.1 = self.fade;
            for _ in 0..due {
                m.effects.tick(&mut self.mix);
                music.tick(&mut self.tune);
            }
        }
        // The game before's music, on its way out.
        let mut left = (0.0, 0.0);
        if let Some((music, level)) = &mut self.leaving {
            left.0 = *level;
            *level = (*level - elapsed.as_secs_f32() / FADE).max(0.0);
            left.1 = *level;
            for _ in 0..due {
                music.tick(&mut self.faded);
            }
            if *level == 0.0 {
                self.leaving = None;
            }
        }
        // (The battle's own sound: its music and its effects are set apart inside it, `battle_levels`.)
        let most = self.music.max(self.effects);
        for s in battle.iter_mut() {
            s[0] *= most;
            s[1] *= most;
        }
        if ticks.is_none() {
            battle.clear();
            battle.resize(self.mix.len(), [0.0; 2]);
        }
        // (Added sample for sample; a sample the menus' drivers made past the battle's frames is let go, so the
        // stream keeps the battle's rate. The music's level goes from the last frame's to this one's across
        // the frame, so a fade has no steps.)
        let count = battle.len().max(1) as f32;
        for (i, b) in battle.iter_mut().enumerate() {
            let effect = self.mix.get(i).copied().unwrap_or_default();
            let tune = self.tune.get(i).copied().unwrap_or_default();
            let old = self.faded.get(i).copied().unwrap_or_default();
            let along = i as f32 / count;
            let level = self.music * (fade.0 + (fade.1 - fade.0) * along);
            let leaving = self.music * (left.0 + (left.1 - left.0) * along);
            b[0] += effect[0] * self.effects + tune[0] * level + old[0] * leaving;
            b[1] += effect[1] * self.effects + tune[1] * level + old[1] * leaving;
        }
        // (`NETTAI_PLAY_STATS`: what can't be seen in a screenshot, said each second.)
        if let Some((since, tune, stream)) = &mut self.stats {
            let peak = |samples: &[[f32; 2]], from: f32| samples.iter().fold(from, |p, s| p.max(s[0].abs()).max(s[1].abs()));
            (*tune, *stream) = (peak(&self.tune, *tune), peak(battle, *stream));
            if now.saturating_duration_since(*since) >= Duration::from_secs(1) {
                eprintln!("sound stats: the title music at {:.2} of its level (its samples' peak {tune:.3}); the stream's peak {stream:.3}", self.fade);
                (*since, *tune, *stream) = (now, 0.0, 0.0);
            }
        }
        if let Some(out) = &self.out
            && !battle.is_empty()
        {
            out.queue(battle);
        }
        battle.clear();
    }
}
