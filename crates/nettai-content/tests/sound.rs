//! Sound packs on a synthetic bank (no game data): songs through MIDI with
//! every case the mapping has an answer for, instruments through TOML and
//! WAV, rendered PCM compared sample for sample, and what the importer says
//! when a DAW or audio editor has been at the files.

use nettai_content::midi::{Message, Smf};
use nettai_content::report::{Level, Report};
use nettai_content::wav::Wav;
use nettai_content::{pack, verify};
use m4a::SongId;
use m4a::bank::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use Command::{Call, EchoLength, EchoVolume, EndTie, Fine, Goto, KeyShift, Note, Pan, Priority, Repeat, Return, Tempo, Wait};
use Command::{BendRange, LfoDelay, LfoSpeed, Modulation, ModulationType, Tune, Volume};
use Command::{Bend, MemAcc, Voice as SetVoice};

fn note(gate: u8, key: Option<u8>, velocity: Option<u8>) -> Command {
    Note { gate, key, velocity }
}

fn env(attack: u8, decay: u8, sustain: u8, release: u8) -> Envelope {
    Envelope { attack, decay, sustain, release }
}

fn bank() -> SoundBank {
    let voice = |kind, key, pan, e| Voice { kind, key, pan, length: 0, envelope: e };
    let main = Voicegroup {
        voices: vec![
            voice(VoiceKind::DirectSound { sample: SampleId(0), fixed: false }, 60, None, env(255, 250, 200, 180)),
            voice(VoiceKind::Square1 { duty: 2, sweep: 0x17, fixed: true }, 60, None, env(0, 2, 10, 3)),
            voice(VoiceKind::Wave { wave: WaveId(0), fixed: true }, 60, None, env(1, 1, 8, 2)),
            voice(VoiceKind::Noise { narrow: true }, 60, None, env(0, 1, 0, 0)),
            voice(VoiceKind::Drums { kit: VoicegroupId(1) }, 60, None, env(0, 0, 0, 0)),
            voice(VoiceKind::Split { group: VoicegroupId(2), map: KeyMapId(0) }, 60, None, env(0, 0, 0, 0)),
            voice(VoiceKind::Silent, 60, None, env(0, 0, 0, 0)),
            voice(VoiceKind::DirectSound { sample: SampleId(1), fixed: true }, 48, Some(-20), env(255, 0, 255, 200)),
        ],
    };
    let kit = Voicegroup {
        voices: (0..128)
            .map(|k| match k % 3 {
                0 => voice(VoiceKind::DirectSound { sample: SampleId(1), fixed: false }, 36 + k as u8 % 24, Some(k as i8 - 64), env(255, 200, 0, 100)),
                1 => voice(VoiceKind::Noise { narrow: false }, 60, Some(10), env(0, 1, 0, 0)),
                _ => voice(VoiceKind::Square2 { duty: (k % 4) as u8, fixed: false }, 60, None, env(0, 0, 15, 1)),
            })
            .collect(),
    };
    let split = Voicegroup {
        voices: vec![
            Voice { length: 5, ..voice(VoiceKind::Square2 { duty: 1, fixed: false }, 60, None, env(0, 0, 15, 0)) },
            voice(VoiceKind::DirectSound { sample: SampleId(0), fixed: false }, 60, None, env(200, 240, 128, 200)),
        ],
    };
    let mut map = [0u8; 128];
    map[60..].fill(1);
    // Music: three tracks whose loops settle at different points.
    let track0 = vec![
        KeyShift(0),
        Tempo(75),
        SetVoice(0),
        Volume(100),
        Pan(64),
        Wait(12),
        Call(12), // 6: loop
        Repeat { count: 2, target: 6 },
        note(24, Some(64), Some(100)),
        Wait(24),
        Goto(6),
        Fine,
        note(12, Some(60), Some(90)), // 12: pattern
        Wait(12),
        note(6, None, Some(70)),
        Wait(12),
        Return,
    ];
    let track1 = vec![
        KeyShift(0),
        SetVoice(1),
        Volume(90),
        MemAcc { op: MemOp::Set, address: 0, operand: 3 },
        MemAcc { op: MemOp::AddFromMemory, address: 2, operand: 0 },
        EchoVolume(20),
        EchoLength(4),
        note(0, Some(50), Some(100)), // a tie that the loop's first end-tie releases
        Wait(12),
        EndTie { key: Some(50) }, // 9: loop
        Wait(36),
        note(24, Some(55), Some(80)),
        Wait(12),
        note(0, Some(50), Some(100)), // crosses the loop's end
        Wait(36),
        Goto(9),
    ];
    let track2 = vec![
        KeyShift(-3),
        SetVoice(5),
        Volume(127),
        BendRange(12),
        LfoSpeed(40),
        LfoDelay(6),
        ModulationType(0),
        Modulation(20),
        Tune(70),
        Priority(5),
        note(30, Some(40), Some(127)),
        Wait(6),
        Bend(80),
        Wait(6),
        Tempo(80), // not on track 0: stays in this MIDI track
        note(30, Some(72), None),
        Wait(18),
        Bend(64),
        SetVoice(4),
        note(10, Some(37), Some(60)), // 19: loop (a drum)
        Wait(12),
        note(10, Some(38), None),
        Wait(12),
        Goto(19),
    ];
    // An effect with the unusual cases.
    let effect = vec![
        SetVoice(7),
        Volume(110),
        note(0, Some(60), Some(100)),
        Wait(12),
        EndTie { key: Some(60) }, // a 12-tick tie
        note(120, Some(62), Some(100)), // a gated note over 96 ticks
        Wait(4),
        EndTie { key: Some(70) }, // an end-tie with no tie
        note(48, Some(64), Some(100)), // cut by FINE
        note(0, Some(67), Some(100)), // never ended
        Wait(12),
        Fine,
    ];
    let song = |player, priority, reverb, tracks: Vec<Vec<Command>>| {
        Some(Song {
            player: PlayerId(player),
            priority,
            reverb,
            voicegroup: VoicegroupId(0),
            tracks: tracks.into_iter().map(|commands| Track { commands }).collect(),
        })
    };
    let bank = SoundBank {
        mixer: MixerConfig { mix_rate: 13379, ds_channels: 4, master_volume: 15, reverb: 0, dac_resolution: 1 },
        players: vec![
            PlayerConfig { max_tracks: 4, uses_priority: false, track_order: 1 },
            PlayerConfig { max_tracks: 2, uses_priority: true, track_order: 0 },
        ],
        songs: vec![song(0, 20, Some(40), vec![track0, track1, track2]), song(1, 64, None, vec![effect]), None],
        voicegroups: vec![main, kit, split],
        key_maps: vec![KeyMap(map)],
        samples: vec![
            Sample {
                rate: 13379 * 1024 + 512,
                loop_start: Some(16),
                data: (0..64).map(|i| (((i * 37) % 200) - 100) as i8).collect(),
                tail: (((16 * 37) % 200) - 100) as i8,
            },
            // (A tail that isn't the usual 0.)
            Sample { rate: 8000 * 1024, loop_start: None, data: (0..48).map(|i| if i % 6 < 3 { 90 } else { -90 }).collect(), tail: 33 },
        ],
        waves: vec![Wave(std::array::from_fn(|i| (i % 16) as u8))],
    };
    bank.validate().unwrap();
    bank
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bn6-content-sound-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_pack(dir: &Path, b: &SoundBank) {
    let (files, failures) = pack::export_sound(b, &nettai_content::names::AssetNames::default());
    assert!(failures.is_empty(), "{failures:?}");
    let mut all = vec![pack::manifest("test", "test", None, true)];
    all.extend(files);
    pack::write_files(dir, &all).unwrap();
}

fn import(dir: &Path) -> (Option<SoundBank>, Report) {
    let mut r = Report::default();
    let b = pack::import_sound(dir, &mut r);
    (b, r)
}

/// The same bank plays the same: timelines and rendered PCM.
fn assert_plays_the_same(a: &SoundBank, b: &SoundBank) {
    assert_eq!((&a.voicegroups, &a.samples, &a.waves, &a.key_maps), (&b.voicegroups, &b.samples, &b.waves, &b.key_maps));
    assert_eq!(a.songs.len(), b.songs.len());
    let (a, b) = (Arc::new(a.clone()), Arc::new(b.clone()));
    for (i, (x, y)) in a.songs.iter().zip(&b.songs).enumerate() {
        match (x, y) {
            (Some(x), Some(y)) => {
                assert_eq!(verify::compare_song(x, y), None, "song {i}");
                let (p, q) = (verify::render(&a, SongId(i as u16), 400), verify::render(&b, SongId(i as u16), 400));
                assert!(p.iter().any(|s| s[0] != 0.0), "song {i} renders silence");
                assert_eq!(verify::first_difference(&p, &q), None, "song {i}");
            }
            (None, None) => {}
            _ => panic!("song {i} is in one bank only"),
        }
    }
    let starts = [(0, SongId(0)), (30, SongId(1)), (31, SongId(1))];
    assert_eq!(verify::first_difference(&verify::render_mix(&a, &starts, 300), &verify::render_mix(&b, &starts, 300)), None);
}

#[test]
fn a_bank_plays_the_same_after_the_round_trip() {
    let dir = temp("roundtrip");
    let b = bank();
    write_pack(&dir, &b);
    let (back, r) = import(&dir);
    assert!(!r.has_errors() && r.count(Level::Warning) == 0, "{r}");
    assert_plays_the_same(&b, &back.unwrap());
    // The extensions this bank needs are in the file.
    let midi = Smf::from_bytes(&std::fs::read(dir.join("sound/songs/sound-001.mid")).unwrap()).unwrap();
    let msgs: Vec<&Message> = midi.tracks.iter().flat_map(|t| t.events.iter().map(|e| &e.message)).collect();
    let cc = |c: u8, v: u8| msgs.iter().any(|m| matches!(m, Message::Control { controller, value, .. } if *controller == c && *value == v));
    assert!(cc(103, 1) && cc(103, 2) && cc(104, 70), "tie/gated marks and the bare end-tie");
    assert!(msgs.iter().any(|m| matches!(m, Message::Marker(s) if s == "fine")));
    let music = Smf::from_bytes(&std::fs::read(dir.join("sound/songs/sound-000.mid")).unwrap()).unwrap();
    assert!(music.tracks[0].events.iter().any(|e| e.message == Message::Marker("[".into())));
    assert!(music.tracks[0].events.iter().any(|e| matches!(e.message, Message::Tempo(400_000))), "track 0's tempo in the conductor");
    assert!(music.tracks[3].events.iter().any(|e| matches!(e.message, Message::Tempo(_))), "track 2's tempo stays on its track");
    // MEMACC as mid2agb's controllers: the operation, the address, then the
    // operand that runs it.
    let track1: Vec<(u8, u8)> = music.tracks[2]
        .events
        .iter()
        .filter_map(|e| match e.message {
            Message::Control { controller, value, .. } if (12..=14).contains(&controller) => Some((controller, value)),
            _ => None,
        })
        .collect();
    assert_eq!(track1, [(13, 0), (14, 0), (12, 3), (13, 4), (14, 2), (12, 0)]);
}

fn edit_midi(path: &Path, f: impl FnOnce(&mut Smf)) {
    let mut smf = Smf::from_bytes(&std::fs::read(path).unwrap()).unwrap();
    f(&mut smf);
    std::fs::write(path, smf.to_bytes()).unwrap();
}

#[test]
fn a_daw_save_at_another_resolution_is_exact() {
    let dir = temp("ppq");
    let b = bank();
    write_pack(&dir, &b);
    for song in ["sound-000", "sound-001"] {
        edit_midi(&dir.join(format!("sound/songs/{song}.mid")), |smf| {
            smf.division = 480;
            for t in &mut smf.tracks {
                t.end *= 20;
                for e in &mut t.events {
                    e.tick *= 20;
                }
            }
        });
    }
    let (back, r) = import(&dir);
    assert!(!r.has_errors(), "{r}");
    assert!(r.issues.iter().any(|i| i.message.contains("edited since export")));
    assert_plays_the_same(&b, &back.unwrap());
}

#[test]
fn damage_to_songs_is_reported() {
    let dir = temp("songdamage");
    write_pack(&dir, &bank());
    let music = dir.join("sound/songs/sound-000.mid");
    let good = std::fs::read(&music).unwrap();
    // Markers and M4A's controllers dropped, as editors that don't know them do.
    edit_midi(&music, |smf| {
        for t in &mut smf.tracks {
            t.events.retain(|e| !matches!(&e.message, Message::Marker(_) | Message::Control { controller: 20..=33 | 102.., .. }));
        }
    });
    let (_, r) = import(&dir);
    let says = |s: &str| r.issues.iter().any(|i| i.level == Level::Warning && i.message.contains(s));
    assert!(says("loop markers"), "{r}");
    assert!(says("all 1 lfo_delay commands are gone") && says("key_shift"), "{r}");
    // The effect's `fine` marker dropped: it would end when its last note does.
    edit_midi(&dir.join("sound/songs/sound-001.mid"), |smf| {
        for t in &mut smf.tracks {
            t.events.retain(|e| e.message != Message::Marker("fine".into()));
        }
    });
    let (_, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.message.contains("now end (FINE) at ticks [132], not [28]")), "{r}");
    // Notes moved off the 24-a-beat grid.
    std::fs::write(&music, &good).unwrap();
    edit_midi(&music, |smf| {
        smf.division = 96;
        for t in &mut smf.tracks {
            t.end *= 4;
            for e in &mut t.events {
                e.tick = e.tick * 4 + matches!(e.message, Message::NoteOn { .. }) as u32;
            }
        }
    });
    let (back, r) = import(&dir);
    assert!(back.is_none() && r.issues.iter().any(|i| i.message.contains("quantize")), "{r}");
}

#[test]
fn audio_editors_lose_loop_chunks_but_not_the_sample() {
    let dir = temp("wav");
    let b = bank();
    write_pack(&dir, &b);
    let path = dir.join("sound/samples/smp-000.wav");
    let w = Wav::from_bytes(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!((w.rate, w.loop_points, w.unity_key), (13380, Some((16, 63)), Some(60)));
    // Saved again as 16-bit without the smpl chunk (Audacity, sox, ffmpeg).
    std::fs::write(&path, Wav { bits: 16, loop_points: None, unity_key: None, ..w.clone() }.to_bytes()).unwrap();
    let (back, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.level == Level::Warning && i.message.contains("smpl")), "{r}");
    let back = back.unwrap();
    assert_eq!(back.samples, b.samples, "the rate's fraction and the loop come from samples.toml");
    // Resampled: the new rate counts, with a warning.
    std::fs::write(&path, Wav { rate: 22050, ..w.clone() }.to_bytes()).unwrap();
    let (back, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.message.contains("rate changed")), "{r}");
    assert_eq!(back.unwrap().samples[0].rate, 22050 * 1024);
    // Stereo is refused.
    let mut stereo = w.to_bytes();
    stereo[22] = 2;
    std::fs::write(&path, stereo).unwrap();
    let (_, r) = import(&dir);
    assert!(r.issues.iter().any(|i| i.message.contains("mono")), "{r}");
}

#[test]
fn songs_mixing_loops_and_endings_are_refused_for_now() {
    let mut b = bank();
    let s = b.songs[0].as_mut().unwrap();
    s.tracks[1] = Track { commands: vec![SetVoice(1), note(12, Some(60), Some(100)), Wait(12), Fine] };
    let (_, failures) = pack::export_sound(&b, &nettai_content::names::AssetNames::default());
    assert_eq!(failures.len(), 1);
    assert!(failures[0].1.contains("one loop per song"));
}

/// Songs a version has its own of (BN5's Team Colonel) are song files of their
/// own named with their version, beside the base version's, and read back as
/// they were; a pack without versions names no version anywhere.
#[test]
fn version_songs_round_trip() {
    use nettai_content::sound::SongVersions;
    let dir = temp("versions");
    let b = bank();
    // Song 1 as the "colonel" version has it: song 0's tracks.
    let other = b.songs[0].clone().unwrap();
    let versions = SongVersions {
        base_version: "protoman".into(),
        versions: vec![("colonel".into(), std::collections::BTreeMap::from([(1u16, other.clone())]))],
    };
    let (files, failures) = pack::export_sound_versions(&b, &versions, &nettai_content::names::AssetNames::default());
    assert!(failures.is_empty(), "{failures:?}");
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).filter(|n| n.starts_with("sound/songs/")).collect();
    for n in ["sound/songs/sound-000.toml", "sound/songs/sound-001-protoman.toml", "sound/songs/sound-001-colonel.toml"] {
        assert!(names.contains(&n), "{n} in {names:?}");
    }
    let mut all = vec![pack::manifest("test", "test", None, true)];
    all.extend(files);
    pack::write_files(&dir, &all).unwrap();
    let mut r = Report::default();
    let (back, back_versions) = pack::import_sound_versions(&dir, &mut r).expect("the pack loads");
    assert!(!r.has_errors() && r.count(Level::Warning) == 0, "{r}");
    assert_plays_the_same(&b, &back);
    assert_eq!(back_versions.base_version, "protoman");
    let colonel = back_versions.song("colonel", 1).expect("colonel's song 1");
    assert_eq!(verify::compare_song(colonel, &other), None);
    // Without versions, no file names one.
    let (plain, _) = pack::export_sound(&b, &nettai_content::names::AssetNames::default());
    let says_version = |d: &[u8]| String::from_utf8_lossy(d).lines().any(|l| l.starts_with("version = \"") || l.starts_with("base_version"));
    assert!(plain.iter().filter(|(n, _)| n.ends_with(".toml")).all(|(_, d)| !says_version(d)));
}
