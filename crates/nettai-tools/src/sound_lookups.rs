//! The audio's lookups: a sound cue's song in its pack's sound. Noted and
//! checked once a run as the drawing code's are (`nettai_render::lookups`,
//! `nettai_render::audit`), in the same `Problems`.

use nettai_battle::content::Content;
use nettai_battle::{Battle, SoundCue};
use nettai_content_api::AssetKind;
use nettai_render::audit::{Lookup, Problems};

/// Check that the pack's sound has the song a cue starts.
pub fn check_cue(b: &Battle, banks: &[std::sync::Arc<m4a::SoundBank>], cue: SoundCue, problems: &mut Problems) {
    match cue {
        SoundCue::Effect(id) => sound(&b.content, banks, id.0, false, problems),
        SoundCue::Music(id) => sound(&b.content, banks, id.0, true, problems),
        _ => {}
    }
}

/// A sound's song in its pack's bank (`banks` by `PackId`), for an effect
/// or music (`music`: the no-music song stops the music and has none).
pub fn sound(c: &Content, banks: &[std::sync::Arc<m4a::SoundBank>], h: u16, music: bool, problems: &mut Problems) {
    if !problems.lookup(Lookup::Sound(h)) {
        return;
    }
    // (The engine's sound is a handle; the song is its pack's, in that
    // pack's bank: the first's for a frontend of one pack.)
    let Some(a) = c.assets.sound(h) else {
        problems.note(format!("sound handle {h} names no sound"));
        return;
    };
    if music && a.id == nettai_audio::NO_MUSIC.0 {
        return;
    }
    let bank = banks.get(a.pack.index()).or(banks.first()).expect("a pack's sound");
    if bank.song(m4a::SongId(a.id)).is_some_and(|s| !s.tracks.is_empty()) {
        return;
    }
    let name = match nettai_render::packs::name(c, AssetKind::Sound, h) {
        Some(name) => format!("sound {name:?} ({:#05x})", a.id),
        None => format!("sound {:#05x}", a.id),
    };
    problems.note(format!("{name} has no song in the pack's sound"));
}
