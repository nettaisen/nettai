//! The sound: the US Team ProtoMan ROM's m4a bank, with any song the US Team
//! Colonel ROM plays otherwise at the same number as Team Colonel's version
//! of it. A Team Colonel song plays with voicegroups the bank has: its own
//! voicegroups, with their samples, PSG waves and key maps, are matched to
//! equal ones of Team ProtoMan's bank or added to it.
//!
//! The two US ROMs play every song alike (`plays_alike`), so the pack has no
//! version's own. Eleven songs' voicegroups (0x13C to 0x142, 0x145, 0x146,
//! 0x170, 0x171) differ between the ROMs only in voices no note of theirs
//! plays: a voicegroup is read 128 voices long, past the voices the game
//! defines for it into the tables and data after them, where the two ROMs'
//! bytes (pointers, at other addresses in each) read as other voices.

use m4a::SoundBank;
use m4a::bank::{Command, KeyMapId, SampleId, Song, Voice, VoiceKind, Voicegroup, VoicegroupId, WaveId};
use nettai_content::sound::SongVersions;
use std::collections::{BTreeMap, HashMap};

/// Team Colonel's own songs (those that play otherwise than Team
/// ProtoMan's at the same number), brought into `bank` (Team ProtoMan's):
/// what `bank` needs for them is added to it. None: no versions.
pub fn colonel_songs(bank: &mut SoundBank, colonel: &SoundBank) -> SongVersions {
    let differ: Vec<u16> = (0..bank.songs.len().max(colonel.songs.len()) as u16)
        .filter(|&id| match (bank.song(m4a::SongId(id)), colonel.song(m4a::SongId(id))) {
            (Some(a), Some(b)) => !plays_alike(bank, a, colonel, b),
            (None, Some(_)) => true,
            (_, None) => false,
        })
        .collect();
    if differ.is_empty() {
        return SongVersions::default();
    }
    let mut merge = Merge { into: bank, from: colonel, groups: HashMap::new() };
    let mut own = BTreeMap::new();
    for id in differ {
        let song = colonel.song(m4a::SongId(id)).expect("a song");
        let voicegroup = merge.group(song.voicegroup);
        own.insert(id, Song { voicegroup, ..song.clone() });
    }
    SongVersions { base_version: "protoman".into(), versions: vec![("colonel".into(), own)] }
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
        if let Some(i) = (0..self.into.voicegroups.len()).find(|&i| same_group(self.from, g.0, self.into, i as u16, &mut HashMap::new())) {
            let m = VoicegroupId(i as u16);
            self.groups.insert(g.0, m);
            return m;
        }
        // Added: reserved first, so a drum kit or split naming it back finds it.
        let m = VoicegroupId(self.into.voicegroups.len() as u16);
        self.into.voicegroups.push(Voicegroup { voices: Vec::new() });
        self.groups.insert(g.0, m);
        let voices: Vec<Voice> = self.from.voicegroups[g.0 as usize].voices.clone();
        let voices = voices.into_iter().map(|v| Voice { kind: self.kind(v.kind), ..v }).collect();
        self.into.voicegroups[m.0 as usize] = Voicegroup { voices };
        m
    }

    fn kind(&mut self, k: VoiceKind) -> VoiceKind {
        match k {
            VoiceKind::DirectSound { sample, fixed } => VoiceKind::DirectSound { sample: self.sample(sample), fixed },
            VoiceKind::Wave { wave, fixed } => VoiceKind::Wave { wave: self.wave(wave), fixed },
            VoiceKind::Drums { kit } => VoiceKind::Drums { kit: self.group(kit) },
            VoiceKind::Split { group, map } => VoiceKind::Split { group: self.group(group), map: self.key_map(map) },
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
fn same_group(ab: &SoundBank, a: u16, bb: &SoundBank, b: u16, seen: &mut HashMap<(u16, u16), bool>) -> bool {
    if let Some(&r) = seen.get(&(a, b)) {
        return r;
    }
    seen.insert((a, b), true);
    let (va, vb) = (&ab.voicegroups[a as usize].voices, &bb.voicegroups[b as usize].voices);
    let r = va.len() == vb.len() && va.iter().zip(vb).all(|(x, y)| same_voice(ab, x, bb, y, seen));
    seen.insert((a, b), r);
    r
}

/// Whether voice `x` of bank `ab` plays as voice `y` of bank `bb`.
fn same_voice(ab: &SoundBank, x: &Voice, bb: &SoundBank, y: &Voice, seen: &mut HashMap<(u16, u16), bool>) -> bool {
    (x.key, x.pan, x.length, x.envelope) == (y.key, y.pan, y.length, y.envelope)
        && match (x.kind, y.kind) {
            (VoiceKind::DirectSound { sample: s, fixed: f }, VoiceKind::DirectSound { sample: t, fixed: g }) => {
                f == g && ab.samples[s.0 as usize] == bb.samples[t.0 as usize]
            }
            (VoiceKind::Wave { wave: w, fixed: f }, VoiceKind::Wave { wave: z, fixed: g }) => f == g && ab.waves[w.0 as usize] == bb.waves[z.0 as usize],
            (VoiceKind::Drums { kit: k }, VoiceKind::Drums { kit: l }) => same_group(ab, k.0, bb, l.0, seen),
            (VoiceKind::Split { group: g, map: m }, VoiceKind::Split { group: h, map: n }) => {
                ab.key_maps[m.0 as usize] == bb.key_maps[n.0 as usize] && same_group(ab, g.0, bb, h.0, seen)
            }
            (x, y) => x == y,
        }
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
    if (a.player, a.priority, a.reverb, &a.tracks) != (b.player, b.priority, b.reverb, &b.tracks) {
        return false;
    }
    let (va, vb) = (&ab.voicegroups[a.voicegroup.0 as usize].voices, &bb.voicegroups[b.voicegroup.0 as usize].voices);
    a.tracks.iter().all(|t| {
        // The keys its notes name; a note without one before any has the
        // track's first key, 0.
        let notes = || t.commands.iter().filter_map(|c| if let Command::Note { key, .. } = *c { Some(key) } else { None });
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
        let plays = |v: &&Voice| !matches!(v.kind, VoiceKind::Drums { .. } | VoiceKind::Split { .. } | VoiceKind::Silent);
        match (p.filter(plays), q.filter(plays)) {
            (Some(p), Some(q)) => same_voice(ab, p, bb, q, &mut HashMap::new()),
            (p, q) => p.is_none() && q.is_none(),
        }
    };
    match (x.kind, y.kind) {
        (VoiceKind::Drums { kit: k }, VoiceKind::Drums { kit: l }) => {
            let (ka, kb) = (&ab.voicegroups[k.0 as usize].voices, &bb.voicegroups[l.0 as usize].voices);
            keys.iter().all(|&key| leaf(ka.get(key as usize), kb.get(key as usize)))
        }
        (VoiceKind::Split { group: g, map: m }, VoiceKind::Split { group: h, map: n }) => {
            let (ga, gb) = (&ab.voicegroups[g.0 as usize].voices, &bb.voicegroups[h.0 as usize].voices);
            keys.iter().all(|&key| {
                let (i, j) = (ab.key_maps[m.0 as usize].0[(key & 0x7F) as usize], bb.key_maps[n.0 as usize].0[(key & 0x7F) as usize]);
                i == j && leaf(ga.get(i as usize), gb.get(j as usize))
            })
        }
        (VoiceKind::Silent, VoiceKind::Silent) => true,
        _ => same_voice(ab, x, bb, y, &mut HashMap::new()),
    }
}
