//! The sound (an `m4a::SoundBank`) as files: songs as MIDI (see [`crate::song`]), instruments
//! as TOML, samples as WAV.
//!
//! ```text
//! sound/
//!   sound.toml            mixer, music players, song table size
//!   samples.toml          per sample: file, exact rate, loop start, stamp
//!   samples/smp-NNN.wav   8-bit mono PCM, `smpl` chunk: unity key 60, loop
//!   waves.toml            PSG wave channel shapes: 32 hex digits each
//!   keymaps.toml          key splits: [first key, last key, voice] ranges
//!   voicegroups/vg-NNN.toml  128 voices (drum kits and split groups too)
//!   songs/NAME.mid        a song, under its name (crate::names)
//!   songs/NAME.toml       its header: song id, player, priority, reverb, voicegroup
//! ```
//!
//! A WAV header can't hold the GBA's sample rate (Hz in 1/1024 steps), and
//! editors drop `smpl` chunks, so samples.toml keeps both, with a stamp of
//! the WAV as exported: an untouched WAV takes them from there exactly; an
//! edited one is read for what it says, and the importer explains what it
//! had to assume.

use crate::report::{Report, stamp};
use crate::song::{self, SongDoc};
use crate::wav::Wav;
use m4a::bank::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

pub const FORMAT: &str = "nettai-content/sound";
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub struct SoundDoc {
    pub format: String,
    pub version: u32,
    /// Entries in the song table (ids without a song file play nothing).
    pub song_table: usize,
    /// The game version the table's songs are, when another version has
    /// songs of its own at the same numbers (`SongVersions`); absent
    /// otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_version: Option<String>,
    pub mixer: MixerDoc,
    pub players: Vec<PlayerDoc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MixerDoc {
    /// Direct Sound mixing rate, Hz.
    pub mix_rate: u32,
    pub channels: u8,
    /// 0..=15.
    pub master_volume: u8,
    /// Reverb before a song sets one (0 = off).
    pub reverb: u8,
    /// The DAC's resolution: 0 is 9 bits at 32768 Hz, 1 is 8 bits at 65536 Hz,
    /// 2 and 3 are 7 and 6 bits at twice and four times that.
    #[serde(default)]
    pub dac_resolution: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerDoc {
    pub max_tracks: u8,
    pub uses_priority: bool,
    pub track_order: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SampleDoc {
    pub file: String,
    /// Playback rate at key 60 in Hz (exact: a multiple of 1/1024).
    pub rate_hz: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_start: Option<u32>,
    /// The byte after the data, which the mixer reads to interpolate the
    /// last sample, if it isn't the usual one (the loop start's sample, or 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tail: Option<i8>,
    /// FNV-1a of the WAV as exported.
    pub stamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VoiceDoc {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duty: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrow: Option<bool>,
    /// Square 1's sweep (NR10; absent: none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sweep: Option<u8>,
    /// A PSG voice's frequency rounds to what the DAC plays exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<bool>,
    /// A PSG voice's sound length (NRx1; absent: until it stops).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keymap: Option<String>,
    /// The key a drum kit's instrument plays at.
    pub key: u8,
    /// A drum kit instrument's own pan (-128..=126).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pan: Option<i8>,
    pub attack: u8,
    pub decay: u8,
    pub sustain: u8,
    pub release: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoicegroupDoc {
    pub voices: Vec<VoiceDoc>,
}

fn name(prefix: &str, i: usize) -> String {
    format!("{prefix}-{i:03}")
}

/// Names in bank order: `prefix-NNN` by number, then any others by name.
fn ordered<'a>(names: impl Iterator<Item = &'a String>, prefix: &str) -> Vec<String> {
    let mut v: Vec<(u64, String)> = names
        .map(|n| {
            let num = n.strip_prefix(prefix).and_then(|r| r.strip_prefix('-')).and_then(|r| r.parse().ok());
            (num.unwrap_or(u64::MAX), n.clone())
        })
        .collect();
    v.sort();
    v.into_iter().map(|x| x.1).collect()
}

/// Songs game versions have their own of at the same numbers as the bank's
/// (no game's ROMs so far: EXE5's two play every song alike). The
/// bank's own songs are `base_version`'s; each other version's play with the
/// bank's voicegroups. A console of a version plays its own, else the
/// bank's. In a pack each is a song file of its own, named with its version
/// (`sound-13c-protoman`, `sound-13c-colonel`), as `nettai_assets::Versioned`
/// names a version's own pictures; the asset index keeps one name a number.
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

// ---- Export ----------------------------------------------------------------------

/// Files of the `sound/` folder; songs that can't be expressed yet are
/// left out with the reason.
pub fn export(bank: &SoundBank, names: &crate::names::AssetNames) -> (crate::pack::Files, Vec<(SongId, String)>) {
    export_with_versions(bank, &SongVersions::default(), names)
}

/// [`export`], with the songs versions have their own of.
pub fn export_with_versions(
    bank: &SoundBank,
    versions: &SongVersions,
    names: &crate::names::AssetNames,
) -> (crate::pack::Files, Vec<(SongId, String)>) {
    let mut files = Vec::new();
    let doc = SoundDoc {
        format: FORMAT.into(),
        version: VERSION,
        song_table: bank.songs.len(),
        base_version: (!versions.versions.is_empty()).then(|| versions.base_version.clone()),
        mixer: MixerDoc {
            mix_rate: bank.mixer.mix_rate,
            channels: bank.mixer.ds_channels,
            master_volume: bank.mixer.master_volume,
            reverb: bank.mixer.reverb,
            dac_resolution: bank.mixer.dac_resolution,
        },
        players: bank
            .players
            .iter()
            .map(|p| PlayerDoc { max_tracks: p.max_tracks, uses_priority: p.uses_priority, track_order: p.track_order })
            .collect(),
    };
    files.push(("sound.toml".into(), toml::to_string_pretty(&doc).unwrap().into_bytes()));
    // Samples.
    let mut samples = BTreeMap::new();
    for (i, s) in bank.samples.iter().enumerate() {
        let n = name("smp", i);
        let file = format!("samples/{n}.wav");
        let wav = Wav::from_pcm8(&s.data, ((s.rate as f64) / 1024.0).round() as u32, s.loop_start).to_bytes();
        samples.insert(
            n,
            SampleDoc {
                file: file.clone(),
                rate_hz: s.rate as f64 / 1024.0,
                loop_start: s.loop_start,
                tail: (s.tail != Sample::usual_tail(&s.data, s.loop_start)).then_some(s.tail),
                stamp: stamp(&wav),
            },
        );
        files.push((file, wav));
    }
    files.push(("samples.toml".into(), toml::to_string_pretty(&BTreeMap::from([("samples", samples)])).unwrap().into_bytes()));
    // Waves and key maps.
    let mut waves = String::from("# PSG wave channel shapes: 32 steps of 0..=15, as hex digits.\n");
    for (i, w) in bank.waves.iter().enumerate() {
        let hex: String = w.0.iter().map(|&x| char::from_digit(x as u32 & 15, 16).unwrap()).collect();
        writeln!(waves, "{} = \"{hex}\"", name("wave", i)).unwrap();
    }
    files.push(("waves.toml".into(), waves.into_bytes()));
    let mut keymaps = String::from("# Key splits: [first key, last key, voice of the split's group].\n");
    for (i, m) in bank.key_maps.iter().enumerate() {
        let mut ranges = Vec::new();
        let mut start = 0;
        for k in 1..=128 {
            if k == 128 || m.0[k] != m.0[start] {
                ranges.push(format!("[{}, {}, {}]", start, k - 1, m.0[start]));
                start = k;
            }
        }
        writeln!(keymaps, "{} = [{}]", name("km", i), ranges.join(", ")).unwrap();
    }
    files.push(("keymaps.toml".into(), keymaps.into_bytes()));
    // Voicegroups.
    for (g, group) in bank.voicegroups.iter().enumerate() {
        let mut s = format!("# Voicegroup {g}: voice n is program n.\nvoices = [\n");
        for (i, v) in group.voices.iter().enumerate() {
            writeln!(s, "  {}, # {i}", inline(&voice_doc(v))).unwrap();
        }
        s.push_str("]\n");
        files.push((format!("voicegroups/{}.toml", name("vg", g)), s.into_bytes()));
    }
    // Songs: the bank's, then each version's own. A song some version has its
    // own of is named with its version, the bank's with the base version.
    let mut failures = Vec::new();
    let own = versions.versions.iter().flat_map(|(v, songs)| songs.iter().map(move |(&id, s)| (id as usize, s, Some(v.as_str()))));
    let all = bank.songs.iter().enumerate().filter_map(|(id, s)| s.as_ref().map(|s| (id, s, None))).chain(own);
    for (id, song, own_version) in all {
        let version = own_version.or((versions.has(id as u16)).then_some(versions.base_version.as_str()));
        let base = match version {
            Some(v) => format!("{}-{v}", names.song(id as u16)),
            None => names.song(id as u16),
        };
        let vg = name("vg", song.voicegroup.0 as usize);
        match song::export(song, id as u16, &format!("Song {id:#05x}"), &vg, &format!("{base}.mid")) {
            Ok((midi, mut doc)) => {
                doc.version = version.map(String::from);
                files.push((format!("songs/{base}.mid"), midi));
                let text = format!(
                    "# Song {id:#05x}: the MIDI file's header data. midi_stamp tells an untouched file from an edited one.\n{}",
                    toml::to_string_pretty(&doc).unwrap()
                );
                files.push((format!("songs/{base}.toml"), text.into_bytes()));
            }
            Err(e) => failures.push((SongId(id as u16), e)),
        }
    }
    (files, failures)
}

fn voice_doc(v: &Voice) -> VoiceDoc {
    let e = v.envelope;
    let mut d = VoiceDoc {
        key: v.key,
        pan: v.pan,
        length: (v.length != 0).then_some(v.length),
        attack: e.attack,
        decay: e.decay,
        sustain: e.sustain,
        release: e.release,
        ..Default::default()
    };
    d.kind = match v.kind {
        VoiceKind::DirectSound { sample, fixed } => {
            d.sample = Some(name("smp", sample.0 as usize));
            if fixed { "direct_sound_fixed" } else { "direct_sound" }
        }
        VoiceKind::Square1 { duty, sweep, fixed } => {
            d.duty = Some(duty);
            d.sweep = (sweep != NO_SWEEP).then_some(sweep);
            d.fixed = fixed.then_some(true);
            "square1"
        }
        VoiceKind::Square2 { duty, fixed } => {
            d.duty = Some(duty);
            d.fixed = fixed.then_some(true);
            "square2"
        }
        VoiceKind::Wave { wave, fixed } => {
            d.wave = Some(name("wave", wave.0 as usize));
            d.fixed = fixed.then_some(true);
            "wave"
        }
        VoiceKind::Noise { narrow } => {
            d.narrow = Some(narrow);
            "noise"
        }
        VoiceKind::Drums { kit } => {
            d.kit = Some(name("vg", kit.0 as usize));
            "drums"
        }
        VoiceKind::Split { group, map } => {
            d.group = Some(name("vg", group.0 as usize));
            d.keymap = Some(name("km", map.0 as usize));
            "split"
        }
        VoiceKind::Silent => "silent",
    }
    .into();
    d
}

/// A TOML inline table of a voice's fields, in their order.
fn inline(d: &VoiceDoc) -> String {
    let v = toml::Value::try_from(d).unwrap();
    let t = v.as_table().unwrap();
    let order = [
        "type", "sample", "duty", "wave", "narrow", "sweep", "fixed", "length", "kit", "group", "keymap", "key", "pan",
        "attack", "decay", "sustain", "release",
    ];
    let parts: Vec<String> = order.iter().filter_map(|k| t.get(*k).map(|x| format!("{k} = {x}"))).collect();
    format!("{{ {} }}", parts.join(", "))
}

// ---- Import ----------------------------------------------------------------------

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path, name: &str, report: &mut Report) -> Option<T> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            report.error(name, format!("can't read: {e}"));
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            report.error(name, format!("invalid: {e}"));
            None
        }
    }
}

fn files_in(dir: &Path, ext: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension().and_then(|x| x.to_str()) == Some(ext)).then(|| p.file_stem()?.to_str().map(String::from))?
        })
        .collect();
    v.sort();
    v
}

pub fn import(dir: &Path, prefix: &str, report: &mut Report) -> Option<SoundBank> {
    import_with_versions(dir, prefix, report).map(|(bank, _)| bank)
}

/// [`import`], with the songs versions have their own of.
pub fn import_with_versions(dir: &Path, prefix: &str, report: &mut Report) -> Option<(SoundBank, SongVersions)> {
    let f = |n: &str| format!("{prefix}/{n}");
    let doc: SoundDoc = read_toml(&dir.join("sound.toml"), &f("sound.toml"), report)?;
    if doc.format != FORMAT || doc.version != VERSION {
        report.error(f("sound.toml"), format!("not a {FORMAT} file of version {VERSION} (extract the pack again)"));
        return None;
    }
    let mixer = MixerConfig {
        mix_rate: doc.mixer.mix_rate,
        ds_channels: doc.mixer.channels,
        master_volume: doc.mixer.master_volume,
        reverb: doc.mixer.reverb,
        dac_resolution: doc.mixer.dac_resolution & 3,
    };
    let players = doc
        .players
        .iter()
        .map(|p| PlayerConfig { max_tracks: p.max_tracks, uses_priority: p.uses_priority, track_order: p.track_order })
        .collect();
    // Samples.
    let sdoc: BTreeMap<String, BTreeMap<String, SampleDoc>> = read_toml(&dir.join("samples.toml"), &f("samples.toml"), report)?;
    let sdoc = sdoc.get("samples").cloned().unwrap_or_default();
    let sample_names = ordered(sdoc.keys(), "smp");
    let mut samples = Vec::new();
    for n in &sample_names {
        samples.push(import_sample(dir, prefix, n, &sdoc[n], report)?);
    }
    let sample_id: HashMap<&str, u16> = sample_names.iter().enumerate().map(|(i, n)| (n.as_str(), i as u16)).collect();
    // Waves and key maps.
    let wdoc: BTreeMap<String, String> = read_toml(&dir.join("waves.toml"), &f("waves.toml"), report)?;
    let wave_names = ordered(wdoc.keys(), "wave");
    let mut waves = Vec::new();
    for n in &wave_names {
        let digits: Vec<u8> = wdoc[n].chars().filter_map(|c| c.to_digit(16).map(|d| d as u8)).collect();
        if digits.len() != 32 {
            report.error(f("waves.toml"), format!("{n}: a wave is 32 hex digits"));
            return None;
        }
        waves.push(Wave(digits.try_into().unwrap()));
    }
    let wave_id: HashMap<&str, u16> = wave_names.iter().enumerate().map(|(i, n)| (n.as_str(), i as u16)).collect();
    let kdoc: BTreeMap<String, Vec<[u8; 3]>> = read_toml(&dir.join("keymaps.toml"), &f("keymaps.toml"), report)?;
    let km_names = ordered(kdoc.keys(), "km");
    let mut key_maps = Vec::new();
    for n in &km_names {
        let mut m = [0u8; 128];
        let mut covered = [false; 128];
        for &[lo, hi, v] in &kdoc[n] {
            for k in lo..=hi.min(127) {
                m[k as usize] = v;
                covered[k as usize] = true;
            }
        }
        if covered.iter().any(|c| !c) {
            report.warn(f("keymaps.toml"), format!("{n}: some keys have no range; they play voice 0"));
        }
        key_maps.push(KeyMap(m));
    }
    let km_id: HashMap<&str, u16> = km_names.iter().enumerate().map(|(i, n)| (n.as_str(), i as u16)).collect();
    // Voicegroups.
    let vg_names = ordered(files_in(&dir.join("voicegroups"), "toml").iter(), "vg");
    let vg_id: HashMap<&str, u16> = vg_names.iter().enumerate().map(|(i, n)| (n.as_str(), i as u16)).collect();
    let mut voicegroups = Vec::new();
    for n in &vg_names {
        let file = f(&format!("voicegroups/{n}.toml"));
        let d: VoicegroupDoc = read_toml(&dir.join(format!("voicegroups/{n}.toml")), &file, report)?;
        let mut voices = Vec::new();
        for (i, v) in d.voices.iter().enumerate() {
            let find = |map: &HashMap<&str, u16>, what: &str, name: &Option<String>, report: &mut Report| -> u16 {
                match name.as_deref().and_then(|x| map.get(x)) {
                    Some(&id) => id,
                    None => {
                        report.error(&file, format!("voice {i}: {what} {name:?} doesn't exist"));
                        0
                    }
                }
            };
            let kind = match v.kind.as_str() {
                "direct_sound" | "direct_sound_fixed" => VoiceKind::DirectSound {
                    sample: SampleId(find(&sample_id, "sample", &v.sample, report)),
                    fixed: v.kind == "direct_sound_fixed",
                },
                "square1" => VoiceKind::Square1 {
                    duty: v.duty.unwrap_or(2) & 3,
                    sweep: v.sweep.unwrap_or(NO_SWEEP),
                    fixed: v.fixed.unwrap_or(false),
                },
                "square2" => VoiceKind::Square2 { duty: v.duty.unwrap_or(2) & 3, fixed: v.fixed.unwrap_or(false) },
                "wave" => VoiceKind::Wave {
                    wave: WaveId(find(&wave_id, "wave", &v.wave, report)),
                    fixed: v.fixed.unwrap_or(false),
                },
                "noise" => VoiceKind::Noise { narrow: v.narrow.unwrap_or(false) },
                "drums" => VoiceKind::Drums { kit: VoicegroupId(find(&vg_id, "voicegroup", &v.kit, report)) },
                "split" => VoiceKind::Split {
                    group: VoicegroupId(find(&vg_id, "voicegroup", &v.group, report)),
                    map: KeyMapId(find(&km_id, "key map", &v.keymap, report)),
                },
                "silent" => VoiceKind::Silent,
                other => {
                    report.error(&file, format!("voice {i}: unknown type {other:?}"));
                    VoiceKind::Silent
                }
            };
            let envelope = Envelope { attack: v.attack, decay: v.decay, sustain: v.sustain, release: v.release };
            let length = v.length.unwrap_or(0);
            voices.push(Voice { kind, key: v.key, pan: v.pan, length, envelope });
        }
        voicegroups.push(Voicegroup { voices });
    }
    // Songs.
    let mut songs: Vec<Option<Song>> = vec![None; doc.song_table];
    let mut versions = SongVersions { base_version: doc.base_version.clone().unwrap_or_default(), versions: Vec::new() };
    for base in files_in(&dir.join("songs"), "toml") {
        let file = f(&format!("songs/{base}.toml"));
        let sd: SongDoc = read_toml(&dir.join(format!("songs/{base}.toml")), &file, report)?;
        let id = sd.id as usize;
        // A version's own song (not the base version's) goes with its version.
        let own_version = sd.version.as_deref().filter(|v| Some(*v) != doc.base_version.as_deref());
        let taken = match own_version {
            Some(v) => versions.song(v, sd.id).is_some(),
            None => songs.get(id).is_some_and(Option::is_some),
        };
        if taken {
            report.error(&file, format!("another song is song {id:#05x} too"));
            continue;
        }
        let Some(&vg) = vg_id.get(sd.voicegroup.as_str()) else {
            report.error(&file, format!("voicegroup {:?} doesn't exist", sd.voicegroup));
            continue;
        };
        let midi_name = f(&format!("songs/{}", sd.midi));
        let bytes = match std::fs::read(dir.join("songs").join(&sd.midi)) {
            Ok(b) => b,
            Err(e) => {
                report.error(&midi_name, format!("can't read: {e}"));
                continue;
            }
        };
        if sd.player as usize >= doc.players.len() {
            report.error(&file, format!("player {} doesn't exist", sd.player));
            continue;
        }
        let song = song::import(&bytes, &sd, VoicegroupId(vg), &midi_name, report);
        match own_version {
            Some(v) => {
                if let Some(song) = song {
                    match versions.versions.iter_mut().find(|(name, _)| name == v) {
                        Some((_, own)) => {
                            own.insert(sd.id, song);
                        }
                        None => versions.versions.push((v.to_string(), BTreeMap::from([(sd.id, song)]))),
                    }
                }
            }
            None => {
                if id >= songs.len() {
                    songs.resize(id + 1, None);
                }
                songs[id] = song;
            }
        }
    }
    versions.versions.sort_by(|a, b| a.0.cmp(&b.0));
    let bank = SoundBank { mixer, players, songs, voicegroups, key_maps, samples, waves };
    if let Err(e) = bank.validate() {
        report.error(f("sound.toml"), format!("the bank doesn't hold together: {e}"));
        return None;
    }
    // A version's own songs play with the bank's voicegroups.
    for (v, own) in &versions.versions {
        for (id, song) in own {
            if song.voicegroup.0 as usize >= bank.voicegroups.len() {
                report.error(f("sound.toml"), format!("{v}'s song {id:#05x} names a voicegroup the bank hasn't"));
                return None;
            }
        }
    }
    Some((bank, versions))
}

fn import_sample(dir: &Path, prefix: &str, n: &str, d: &SampleDoc, report: &mut Report) -> Option<Sample> {
    let file = format!("{prefix}/{}", d.file);
    let bytes = match std::fs::read(dir.join(&d.file)) {
        Ok(b) => b,
        Err(e) => {
            report.error(&file, format!("can't read: {e}"));
            return None;
        }
    };
    let wav = match Wav::from_bytes(&bytes) {
        Ok(w) => w,
        Err(e) => {
            report.error(&file, e);
            return None;
        }
    };
    let untouched = stamp(&bytes) == d.stamp;
    let mut rounded = 0;
    let mut data: Vec<i8> = wav
        .samples
        .iter()
        .map(|&s| {
            if s & 0xFF != 0 {
                rounded += 1;
            }
            ((s as i32 + 0x80) >> 8).clamp(-128, 127) as i8
        })
        .collect();
    if data.is_empty() {
        report.error(&file, "no samples");
        return None;
    }
    if rounded > 0 {
        report.warn(&file, format!("{rounded} samples had more than 8 bits of detail; rounded to the GBA's 8-bit PCM"));
    }
    let exact_rate = (d.rate_hz * 1024.0).round() as u32;
    let mut rate = exact_rate;
    let mut loop_start = d.loop_start;
    if !untouched {
        report.note(&file, format!("{n} was edited since export"));
        if wav.rate != (d.rate_hz).round() as u32 {
            report.warn(&file, format!("the rate changed to {} Hz (was {} Hz); samples.toml's rate is ignored", wav.rate, d.rate_hz));
            rate = wav.rate * 1024;
        }
        match wav.loop_points {
            Some((start, end)) => {
                if end as usize + 1 < data.len() {
                    report.warn(&file, format!("the loop ends at sample {end}; M4A loops to the end, so the {} samples after it are cut", data.len() - end as usize - 1));
                    data.truncate(end as usize + 1);
                }
                loop_start = Some(start);
            }
            None if d.loop_start.is_some() => report.warn(
                &file,
                "no loop points in the WAV (editors like Audacity drop the smpl chunk); using samples.toml's loop_start",
            ),
            None => {}
        }
        if let Some(k) = wav.unity_key.filter(|&k| k != 60) {
            rate = (rate as f64 * 2f64.powf((60.0 - k as f64) / 12.0)).round() as u32;
            report.note(&file, format!("unity key {k}: the rate is converted to key 60's"));
        }
    }
    if let Some(l) = loop_start
        && l as usize >= data.len()
    {
        report.error(&file, format!("the loop starts at {l}, past the {} samples", data.len()));
        return None;
    }
    let tail = match d.tail {
        Some(t) if untouched => t,
        _ => Sample::usual_tail(&data, loop_start),
    };
    Some(Sample { rate, loop_start, data, tail })
}
