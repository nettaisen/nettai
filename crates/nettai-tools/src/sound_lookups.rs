//! The audio's lookups: a sound cue's song in the game's sound. Noted and
//! checked once a run as the drawing code's are (`nettai_render::lookups`,
//! `nettai_render::audit`), in the same `Problems`.

use nettai_battle::content::{Content, AssetNumbers};
use nettai_battle::{Battle, SoundCue};
use nettai_content_api::AssetKind;
use nettai_render::audit::{Lookup, Problems};

/// Check that the game's sound has the song a cue starts.
pub fn check_cue(b: &Battle, numbers: &AssetNumbers, bank: &m4a::SoundBank, cue: SoundCue, problems: &mut Problems) {
    match cue {
        SoundCue::Effect(id) => sound(&b.content, numbers, bank, id.0, false, problems),
        SoundCue::Music(id) => sound(&b.content, numbers, bank, id.0, true, problems),
        _ => {}
    }
}

/// A sound's song in the game's bank (`numbers`, the game's numbers for the
/// content's assets), for an effect or music (`music`: the no-music song
/// stops the music and has none).
pub fn sound(c: &Content, numbers: &AssetNumbers, bank: &m4a::SoundBank, h: u16, music: bool, problems: &mut Problems) {
    if !problems.lookup(Lookup::Sound(h)) {
        return;
    }
    // (The engine's sound is a handle; the song is the game's.)
    let Some(song) = numbers.sound(h) else {
        problems.note(format!("sound handle {h} names no sound"));
        return;
    };
    if music && song == nettai_audio::NO_MUSIC.0 {
        return;
    }
    if bank.song(m4a::SongId(song)).is_some_and(|s| !s.tracks.is_empty()) {
        return;
    }
    let name = match nettai_render::graphics::name(c, AssetKind::Sound, h) {
        Some(name) => format!("sound {name:?} ({song:#05x})"),
        None => format!("sound {song:#05x}"),
    };
    problems.note(format!("{name} has no song in the game's sound"));
}
