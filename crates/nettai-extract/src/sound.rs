//! A game's sound versions: the songs a version's ROM plays otherwise than
//! the base version's at the same number, or has alone, brought into the
//! base version's bank as that version's own (`own_songs`; a recipe names
//! the versions and which ROM is each, docs/design/extraction-recipes.md
//! §3.8). A version's song plays with voicegroups the bank has: its own
//! voicegroups, with their samples, PSG waves and key maps, are matched to
//! equal ones of the base bank or added to it. `played_otherwise` compares
//! two banks (the verification's cross-ROM sweep).
//!
//! Two songs play alike when their notes play alike (`plays_alike`), not
//! when their bytes are equal: a voicegroup is read 128 voices long, past
//! the voices the game defines for it into the tables and data after them,
//! where two ROMs' bytes (pointers, at other addresses in each) read as
//! other voices that no note plays.

use m4a::SoundBank;
use m4a::bank::{
    Command, KeyMapId, SampleId, Song, Voice, VoiceKind, Voicegroup, VoicegroupId, WaveId,
};
use std::collections::{BTreeMap, HashMap};

/// The songs each version of a game has its own of: the base version's
/// name (the bank's own songs are its), and each other version's songs that
/// play otherwise than the bank's at the same number, or that only it has,
/// by number, playing with the bank's voicegroups. A console of a version
/// plays its own, else the bank's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SongVersions {
    pub base_version: String,
    pub versions: Vec<(String, BTreeMap<u16, Song>)>,
}

impl SongVersions {
    /// Whether some version has its own song `id`.
    pub fn has(&self, id: u16) -> bool {
        self.versions.iter().any(|(_, songs)| songs.contains_key(&id))
    }

    /// The version's own song `id`, if it has one.
    pub fn song(&self, version: &str, id: u16) -> Option<&Song> {
        self.versions.iter().find(|(v, _)| v == version).and_then(|(_, songs)| songs.get(&id))
    }
}

/// A version's own songs: those `other` (its bank) plays otherwise than
/// `bank` (the base version's) at the same number, or has alone, brought
/// into `bank`: what `bank` needs for them (voicegroups, samples, waves,
/// key maps) is matched to its own or added to it.
pub fn own_songs(bank: &mut SoundBank, other: &SoundBank) -> BTreeMap<u16, Song> {
    let differ: Vec<u16> = (0..bank.songs.len().max(other.songs.len()) as u16)
        .filter(
            |&id| match (bank.song(m4a::SongId(id)), other.song(m4a::SongId(id))) {
                (Some(a), Some(b)) => !plays_alike(bank, a, other, b),
                (None, Some(_)) => true,
                (_, None) => false,
            },
        )
        .collect();
    let mut own = BTreeMap::new();
    if differ.is_empty() {
        return own;
    }
    let mut merge = Merge {
        into: bank,
        from: other,
        groups: HashMap::new(),
    };
    for id in differ {
        let song = other.song(m4a::SongId(id)).expect("a song");
        let voicegroup = merge.group(song.voicegroup);
        own.insert(
            id,
            Song {
                voicegroup,
                ..song.clone()
            },
        );
    }
    own
}

struct Merge<'a> {
    into: &'a mut SoundBank,
    from: &'a SoundBank,
    /// `from`'s voicegroups already brought in, by their index there.
    groups: HashMap<u16, VoicegroupId>,
}

impl Merge<'_> {
    /// `from`'s voicegroup `g` in `into`: an equal one, or a copy added.
    fn group(&mut self, g: VoicegroupId) -> VoicegroupId {
        if let Some(&m) = self.groups.get(&g.0) {
            return m;
        }
        if let Some(i) = (0..self.into.voicegroups.len())
            .find(|&i| same_group(self.from, g.0, self.into, i as u16, &mut HashMap::new()))
        {
            let m = VoicegroupId(i as u16);
            self.groups.insert(g.0, m);
            return m;
        }
        // Added: reserved first, so a drum kit or split naming it back finds it.
        let m = VoicegroupId(self.into.voicegroups.len() as u16);
        self.into
            .voicegroups
            .push(Voicegroup { voices: Vec::new() });
        self.groups.insert(g.0, m);
        let voices: Vec<Voice> = self.from.voicegroups[g.0 as usize].voices.clone();
        let voices = voices
            .into_iter()
            .map(|v| Voice {
                kind: self.kind(v.kind),
                ..v
            })
            .collect();
        self.into.voicegroups[m.0 as usize] = Voicegroup { voices };
        m
    }

    fn kind(&mut self, k: VoiceKind) -> VoiceKind {
        match k {
            VoiceKind::DirectSound { sample, fixed } => VoiceKind::DirectSound {
                sample: self.sample(sample),
                fixed,
            },
            VoiceKind::Wave { wave, fixed } => VoiceKind::Wave {
                wave: self.wave(wave),
                fixed,
            },
            VoiceKind::Drums { kit } => VoiceKind::Drums {
                kit: self.group(kit),
            },
            VoiceKind::Split { group, map } => VoiceKind::Split {
                group: self.group(group),
                map: self.key_map(map),
            },
            other => other,
        }
    }

    fn sample(&mut self, s: SampleId) -> SampleId {
        let x = &self.from.samples[s.0 as usize];
        SampleId(index_of(&mut self.into.samples, x) as u16)
    }

    fn wave(&mut self, w: WaveId) -> WaveId {
        let x = &self.from.waves[w.0 as usize];
        WaveId(index_of(&mut self.into.waves, x) as u16)
    }

    fn key_map(&mut self, m: KeyMapId) -> KeyMapId {
        let x = &self.from.key_maps[m.0 as usize];
        KeyMapId(index_of(&mut self.into.key_maps, x) as u16)
    }
}

/// Where `x` is in `v`, added at the end if it isn't.
fn index_of<T: PartialEq + Clone>(v: &mut Vec<T>, x: &T) -> usize {
    match v.iter().position(|y| y == x) {
        Some(i) => i,
        None => {
            v.push(x.clone());
            v.len() - 1
        }
    }
}

/// Whether voicegroup `a` of bank `ab` plays as voicegroup `b` of bank `bb`:
/// the same voices, with equal samples, waves and key maps and (in turn) the
/// same drum kits and split groups. `seen` assumes a pair under comparison
/// equal (a group can name itself).
fn same_group(
    ab: &SoundBank,
    a: u16,
    bb: &SoundBank,
    b: u16,
    seen: &mut HashMap<(u16, u16), bool>,
) -> bool {
    if let Some(&r) = seen.get(&(a, b)) {
        return r;
    }
    seen.insert((a, b), true);
    let (va, vb) = (
        &ab.voicegroups[a as usize].voices,
        &bb.voicegroups[b as usize].voices,
    );
    let r = va.len() == vb.len()
        && va
            .iter()
            .zip(vb)
            .all(|(x, y)| same_voice(ab, x, bb, y, seen));
    seen.insert((a, b), r);
    r
}

/// Whether voice `x` of bank `ab` plays as voice `y` of bank `bb`.
fn same_voice(
    ab: &SoundBank,
    x: &Voice,
    bb: &SoundBank,
    y: &Voice,
    seen: &mut HashMap<(u16, u16), bool>,
) -> bool {
    (x.key, x.pan, x.length, x.envelope) == (y.key, y.pan, y.length, y.envelope)
        && match (x.kind, y.kind) {
            (
                VoiceKind::DirectSound {
                    sample: s,
                    fixed: f,
                },
                VoiceKind::DirectSound {
                    sample: t,
                    fixed: g,
                },
            ) => f == g && ab.samples[s.0 as usize] == bb.samples[t.0 as usize],
            (VoiceKind::Wave { wave: w, fixed: f }, VoiceKind::Wave { wave: z, fixed: g }) => {
                f == g && ab.waves[w.0 as usize] == bb.waves[z.0 as usize]
            }
            (VoiceKind::Drums { kit: k }, VoiceKind::Drums { kit: l }) => {
                same_group(ab, k.0, bb, l.0, seen)
            }
            (VoiceKind::Split { group: g, map: m }, VoiceKind::Split { group: h, map: n }) => {
                ab.key_maps[m.0 as usize] == bb.key_maps[n.0 as usize]
                    && same_group(ab, g.0, bb, h.0, seen)
            }
            (x, y) => x == y,
        }
}

/// The numbers of the songs bank `b` plays otherwise than bank `a`
/// (`plays_alike`), or that only one of them has.
pub fn played_otherwise(a: &SoundBank, b: &SoundBank) -> Vec<u16> {
    (0..a.songs.len().max(b.songs.len()) as u16)
        .filter(|&id| match (a.song(m4a::SongId(id)), b.song(m4a::SongId(id))) {
            (Some(x), Some(y)) => !plays_alike(a, x, b, y),
            (None, None) => false,
            _ => true,
        })
        .collect()
}

/// Whether song `b` of bank `bb` plays as song `a` of bank `ab`: the same
/// player, priority, reverb and commands, and voices that play alike on
/// the keys of each track's notes, for every voice the track's commands
/// select (`Command::Voice`). (A voicegroup is read 128 voices long,
/// whatever the game defines of it: past its voices the table runs into
/// the data after it, which two ROMs have at other addresses, so two ROMs'
/// copies of one voicegroup, or of one drum kit, differ in a tail no note
/// plays.)
fn plays_alike(ab: &SoundBank, a: &Song, bb: &SoundBank, b: &Song) -> bool {
    (a.player, a.priority, a.reverb, &a.tracks) == (b.player, b.priority, b.reverb, &b.tracks) && voices_alike(ab, a, bb, b)
}

/// Whether the voices `b`'s tracks select play alike in the two songs'
/// voicegroups, on the keys of each track's notes.
fn voices_alike(ab: &SoundBank, a: &Song, bb: &SoundBank, b: &Song) -> bool {
    let (va, vb) = (
        &ab.voicegroups[a.voicegroup.0 as usize].voices,
        &bb.voicegroups[b.voicegroup.0 as usize].voices,
    );
    b.tracks.iter().all(|t| {
        // The keys its notes name; a note without one before any has the
        // track's first key, 0.
        let notes = || {
            t.commands.iter().filter_map(|c| {
                if let Command::Note { key, .. } = *c {
                    Some(key)
                } else {
                    None
                }
            })
        };
        let mut keys: Vec<u8> = notes().flatten().collect();
        if notes().next() == Some(None) {
            keys.push(0);
        }
        keys.sort_unstable();
        keys.dedup();
        t.commands.iter().all(|c| match *c {
            Command::Voice(v) => match (va.get(v as usize), vb.get(v as usize)) {
                (Some(x), Some(y)) => same_played(ab, x, bb, y, &keys),
                (x, y) => x.is_none() && y.is_none(),
            },
            _ => true,
        })
    })
}

/// Whether voice `x` of bank `ab` plays as voice `y` of bank `bb` on notes
/// of `keys`: a drum kit by the voices those keys pick of it, a split by
/// the ones its key map gives them (the driver's note on).
fn same_played(ab: &SoundBank, x: &Voice, bb: &SoundBank, y: &Voice, keys: &[u8]) -> bool {
    // (What a note plays of a kit or a split: nothing for a kit, a split
    // or a silent entry there.)
    let leaf = |p: Option<&Voice>, q: Option<&Voice>| {
        let plays = |v: &&Voice| {
            !matches!(
                v.kind,
                VoiceKind::Drums { .. } | VoiceKind::Split { .. } | VoiceKind::Silent
            )
        };
        match (p.filter(plays), q.filter(plays)) {
            (Some(p), Some(q)) => same_voice(ab, p, bb, q, &mut HashMap::new()),
            (p, q) => p.is_none() && q.is_none(),
        }
    };
    match (x.kind, y.kind) {
        (VoiceKind::Drums { kit: k }, VoiceKind::Drums { kit: l }) => {
            let (ka, kb) = (
                &ab.voicegroups[k.0 as usize].voices,
                &bb.voicegroups[l.0 as usize].voices,
            );
            keys.iter()
                .all(|&key| leaf(ka.get(key as usize), kb.get(key as usize)))
        }
        (VoiceKind::Split { group: g, map: m }, VoiceKind::Split { group: h, map: n }) => {
            let (ga, gb) = (
                &ab.voicegroups[g.0 as usize].voices,
                &bb.voicegroups[h.0 as usize].voices,
            );
            keys.iter().all(|&key| {
                let (i, j) = (
                    ab.key_maps[m.0 as usize].0[(key & 0x7F) as usize],
                    bb.key_maps[n.0 as usize].0[(key & 0x7F) as usize],
                );
                i == j && leaf(ga.get(i as usize), gb.get(j as usize))
            })
        }
        (VoiceKind::Silent, VoiceKind::Silent) => true,
        _ => same_voice(ab, x, bb, y, &mut HashMap::new()),
    }
}
