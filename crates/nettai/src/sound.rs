//! The sound: the audio device, the battle's samples to it, and the menus'
//! sounds, which are the game's own: its custom screen's cursor, pick and
//! taking back, and its refusal, as its rules' sound roles name them (the
//! app names no sound of a pack).

use crate::UiSound;
use nettai_audio::{BattleAudio, Output, SoundCue};
use nettai_battle::content::SoundRole;
use nettai_frontend::game::Loaded;
use nettai_frontend::player::FRAME_RATE;
use std::time::{Duration, Instant};

/// The menus' sounds: a game's sound, and the cues of its roles.
struct Menu {
    audio: BattleAudio,
    cursor: Option<SoundCue>,
    pick: Option<SoundCue>,
    back: Option<SoundCue>,
    refused: Option<SoundCue>,
}

pub struct Sound {
    out: Option<Output>,
    /// Why there is no output, if there isn't.
    pub problem: Option<String>,
    menu: Option<Menu>,
    /// Which game's sounds the menus have.
    menu_game: Option<String>,
    pub menu_sounds: bool,
    last: Option<Instant>,
    owed: f64,
    mix: Vec<[f32; 2]>,
}

impl Sound {
    pub fn open() -> Sound {
        let (out, problem) = match Output::open() {
            Ok(o) => (Some(o), None),
            Err(e) => (None, Some(e.to_string())),
        };
        Sound { out, problem, menu: None, menu_game: None, menu_sounds: true, last: None, owed: 0.0, mix: Vec::new() }
    }

    /// The volume, 0 to 10.
    pub fn set_volume(&self, level: u32) {
        if let Some(o) = &self.out {
            // (The GBA's full scale is loud; 10 is 0.8 of it.)
            o.set_volume(0.08 * level.min(10) as f32);
        }
    }

    /// The menus' sounds from `game` (its id), if they aren't its already.
    pub fn use_game(&mut self, id: &str, game: &Loaded) {
        if self.menu_game.as_deref() == Some(id) {
            return;
        }
        let Some(sound) = &game.sound else { return };
        let roles = &game.game.content.defs.roles().sounds;
        let cue = |role| roles.get(&role).map(|&id| SoundCue::Effect(id));
        self.menu = Some(Menu {
            audio: BattleAudio::with_banks(sound.banks.clone(), sound.songs.clone()),
            cursor: cue(SoundRole::CustomCursor),
            pick: cue(SoundRole::CustomPick),
            back: cue(SoundRole::CustomBack),
            refused: cue(SoundRole::Refused),
        });
        self.menu_game = Some(id.to_string());
    }

    pub fn play(&mut self, s: UiSound) {
        if !self.menu_sounds {
            return;
        }
        let Some(m) = &mut self.menu else { return };
        let cue = match s {
            UiSound::Cursor => m.cursor,
            UiSound::Pick => m.pick,
            UiSound::Back => m.back,
            UiSound::Refused => m.refused,
        };
        if let Some(cue) = cue {
            m.audio.handle(&[cue]);
        }
    }

    /// A frame at `now`: the menus' sound runs at the battle's rate, and
    /// what it made is added to `battle`'s samples (the battle's ticks
    /// since the last frame), which go to the device.
    pub fn frame(&mut self, now: Instant, battle: &mut Vec<[f32; 2]>) {
        let elapsed = self.last.map_or(Duration::ZERO, |l| now.saturating_duration_since(l)).min(Duration::from_millis(250));
        self.last = Some(now);
        self.owed += elapsed.as_secs_f64() * FRAME_RATE;
        self.mix.clear();
        if let Some(m) = &mut self.menu {
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                m.audio.tick(&mut self.mix);
            }
        } else {
            self.owed = self.owed.fract();
        }
        // (The two at the same rate: added where both play, the longer's
        // rest after.)
        for (b, m) in battle.iter_mut().zip(&self.mix) {
            b[0] += m[0];
            b[1] += m[1];
        }
        if self.mix.len() > battle.len() {
            battle.extend_from_slice(&self.mix[battle.len()..]);
        }
        if let Some(out) = &self.out {
            out.queue(battle);
        }
        battle.clear();
    }
}
