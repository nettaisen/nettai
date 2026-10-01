//! What a battle asked the presentation for and the pack didn't have: a
//! sprite, an animation or a palette an object names, a chip without a
//! name or an icon, a banner without glyphs, a sound without a song.
//!
//! Drawing skips what it can't find, so nothing here stops a frame; the
//! renderer and the audio check note each case in [`Problems`], and
//! `--audit` (headless) runs a whole trace and prints them. An empty list
//! says the frames were drawn and the cues played with everything they
//! named, not that they look or sound like the original (the frame
//! comparison outside this repository checks that).

use bn6_battle::{Battle, SoundCue};
use std::collections::BTreeMap;

/// Check that the pack's sound has the song a cue starts.
pub fn check_cue(b: &Battle, bank: &m4a::SoundBank, cue: SoundCue, problems: &mut Problems) {
    let id = match cue {
        SoundCue::Effect(id) => id,
        SoundCue::Music(id) if id != bn6_audio::NO_MUSIC => id,
        _ => return,
    };
    if bank.song(m4a::SongId(id.0)).is_some_and(|s| !s.tracks.is_empty()) {
        return;
    }
    let name = match b.content.assets.sounds.iter().find(|(_, n)| **n == id.0) {
        Some((name, _)) => format!("sound {name:?} ({:#05x})", id.0),
        None => format!("sound {:#05x}", id.0),
    };
    problems.note(format!("{name} has no song in the pack's sound"));
}

/// How often a problem was seen, and on which frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub count: u32,
    /// The first and last frames it was seen on (the driver's frame
    /// numbers; none when it has none).
    pub first: Option<u32>,
    pub last: Option<u32>,
}

/// The problems seen so far, each once, in the order of their text.
#[derive(Clone, Debug, Default)]
pub struct Problems {
    frame: Option<u32>,
    seen: BTreeMap<String, Seen>,
}

impl Problems {
    /// The frame the notes that follow belong to.
    pub fn at(&mut self, frame: Option<u32>) {
        self.frame = frame;
    }

    /// Note a problem (the text names what is missing and who asked).
    pub fn note(&mut self, what: String) {
        let frame = self.frame;
        let s = self.seen.entry(what).or_insert(Seen { count: 0, first: frame, last: frame });
        s.count += 1;
        s.last = frame;
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Seen)> {
        self.seen.iter().map(|(k, &v)| (k.as_str(), v))
    }

    /// Forget everything (a new run).
    pub fn clear(&mut self) {
        *self = Problems::default();
    }

    /// One line per problem: the text, how often, and the frames.
    pub fn lines(&self) -> Vec<String> {
        self.iter()
            .map(|(what, s)| match (s.first, s.last) {
                (Some(a), Some(b)) if a != b => format!("{what} ({} times, frames {a}..={b})", s.count),
                (Some(a), _) => format!("{what} (frame {a})"),
                _ => format!("{what} ({} times)", s.count),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_are_counted_once_each() {
        let mut p = Problems::default();
        assert!(p.is_empty());
        p.at(Some(10));
        p.note("no sprite".into());
        p.at(Some(12));
        p.note("no sprite".into());
        p.note("no icon".into());
        assert_eq!(p.len(), 2);
        assert_eq!(p.lines(), ["no icon (frame 12)", "no sprite (2 times, frames 10..=12)"]);
    }
}
