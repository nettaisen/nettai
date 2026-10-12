//! The sound: the audio device, the battle's samples to it, the menus' sounds and the title music.
//!
//! The menus' sounds are the selected game's own: its custom screen's cursor, pick and taking back, and its
//! refusal, as its rules' sound roles name them (the app names no sound of a pack). The title music is the
//! game's too: the sound its pack names `title-screen`, played across the menu screens and faded out as a
//! battle starts (docs/design/app-ui.md §6).
//!
//! The device gets one stream at the battle's rate. While a battle runs, the menus' sounds are ticked with
//! its ticks and added to its samples; with none running they run on their own clock at the same rate
//! (docs/app.md §1, "The sound").

use nettai_audio::{BattleAudio, Output, SoundCue, SoundId};
use nettai_battle::content::SoundRole;
use nettai_content_api::assets::AssetKind;
use nettai_frontend::game::Loaded;
use nettai_frontend::player::FRAME_RATE;
use std::time::{Duration, Instant};

/// The name a pack gives its title screen's music.
const TITLE_MUSIC: &str = "title-screen";

/// How long the title music takes to fade out.
const FADE: f32 = 0.9;

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
}

/// The selected game's sound for the menus: its effects (the cues of its roles) and its title music, each on a
/// driver of its own so their levels are set apart.
struct Menu {
    effects: BattleAudio,
    cursor: Option<SoundCue>,
    pick: Option<SoundCue>,
    back: Option<SoundCue>,
    refused: Option<SoundCue>,
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
    /// The title music is wanted (a menu screen shows), and how much of it is left as it fades (1 to 0).
    wanted: bool,
    fade: f32,
    last: Option<Instant>,
    owed: f64,
    mix: Vec<[f32; 2]>,
    tune: Vec<[f32; 2]>,
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
        Sound { out, menu: None, game: None, music: 0.8, effects: 1.0, wanted: false, fade: 1.0, last: None, owed: 0.0, mix: Vec::new(), tune: Vec::new() }
    }

    /// No device: nothing is heard (the screenshot pass).
    pub fn silent() -> Sound {
        Sound { out: None, menu: None, game: None, music: 0.8, effects: 1.0, wanted: false, fade: 1.0, last: None, owed: 0.0, mix: Vec::new(), tune: Vec::new() }
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

    /// The menus' sounds and music from `game` (its id), if they aren't its already.
    pub fn use_game(&mut self, id: &str, game: &Loaded) {
        if self.game.as_deref() == Some(id) {
            return;
        }
        let Some(sound) = &game.sound else { return };
        let content = &game.game.content;
        let roles = &content.defs.roles().sounds;
        let cue = |role| roles.get(&role).map(|&id| SoundCue::Effect(id));
        self.menu = Some(Menu {
            effects: BattleAudio::with_banks(sound.banks.clone(), sound.songs.clone()),
            cursor: cue(SoundRole::CustomCursor),
            pick: cue(SoundRole::CustomPick),
            back: cue(SoundRole::CustomBack),
            refused: cue(SoundRole::Refused),
            music: BattleAudio::with_banks(sound.banks.clone(), sound.songs.clone()),
            title: content.assets.handle(AssetKind::Sound, TITLE_MUSIC).map(|h| SoundCue::Music(SoundId(h))),
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
        let mut fade = (self.fade, self.fade);
        if let Some(m) = &mut self.menu {
            // The title music: started when it is wanted, faded and then stopped when it isn't.
            if self.wanted && !m.playing {
                if let Some(title) = m.title {
                    m.music.handle(&[title]);
                    m.playing = true;
                }
                self.fade = 1.0;
                fade = (1.0, 1.0);
            } else if self.wanted {
                self.fade = 1.0;
                fade.1 = 1.0;
            } else if m.playing {
                self.fade = (self.fade - elapsed.as_secs_f32() / FADE).max(0.0);
                fade.1 = self.fade;
                if self.fade == 0.0 {
                    m.music.handle(&[SoundCue::StopMusic]);
                    m.playing = false;
                }
            }
            for _ in 0..due {
                m.effects.tick(&mut self.mix);
                m.music.tick(&mut self.tune);
            }
        }
        for s in battle.iter_mut() {
            s[0] *= self.effects;
            s[1] *= self.effects;
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
            let level = self.music * (fade.0 + (fade.1 - fade.0) * i as f32 / count);
            b[0] += effect[0] * self.effects + tune[0] * level;
            b[1] += effect[1] * self.effects + tune[1] * level;
        }
        if let Some(out) = &self.out
            && !battle.is_empty()
        {
            out.queue(battle);
        }
        battle.clear();
    }
}
