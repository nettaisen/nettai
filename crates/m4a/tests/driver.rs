//! The driver on small banks built in code: music players and priorities,
//! channel competition between players, the player controls, and the
//! sequencer's commands.

use m4a::bank::*;
use m4a::driver::{ChannelInfo, Hardware};
use m4a::{Driver, PlayerId, SongId};
use std::sync::Arc;

use Command::*;

const SQUARE: u8 = 0;
const SAMPLE: u8 = 1;

fn voice(kind: VoiceKind, sustain: u8) -> m4a::bank::Voice {
    m4a::bank::Voice {
        kind,
        key: 60,
        pan: None,
        length: 0,
        envelope: Envelope {
            attack: if matches!(kind, VoiceKind::DirectSound { .. }) { 255 } else { 0 },
            decay: 0,
            sustain,
            release: 0,
        },
    }
}

/// Players: 0 and 1 are effect players (priority rules, 2 tracks), 2 the
/// music player (any song replaces its song, 4 tracks).
fn bank(songs: Vec<(u8, u8, Vec<Vec<Command>>)>, ds_channels: u8) -> Arc<SoundBank> {
    let voices = vec![
        voice(VoiceKind::Square1 { duty: 2, sweep: NO_SWEEP, fixed: false }, 15),
        voice(VoiceKind::DirectSound { sample: SampleId(0), fixed: false }, 255),
    ];
    let songs = songs
        .into_iter()
        .map(|(player, priority, tracks)| {
            Some(Song {
                player: PlayerId(player),
                priority,
                reverb: None,
                voicegroup: VoicegroupId(0),
                tracks: tracks.into_iter().map(|commands| Track { commands }).collect(),
            })
        })
        .collect();
    let players = vec![
        PlayerConfig { max_tracks: 2, uses_priority: true, track_order: 0 },
        PlayerConfig { max_tracks: 2, uses_priority: true, track_order: 1 },
        PlayerConfig { max_tracks: 4, uses_priority: false, track_order: 2 },
    ];
    let sample = Sample {
        rate: 13379 * 1024,
        loop_start: Some(0),
        data: (0..64).map(|i| if i < 32 { 60 } else { -60 }).collect(),
        tail: 60,
    };
    let bank = SoundBank {
        mixer: MixerConfig { mix_rate: 13379, ds_channels, master_volume: 15, reverb: 0, dac_resolution: 1 },
        players,
        songs,
        voicegroups: vec![Voicegroup { voices }],
        key_maps: vec![],
        samples: vec![sample],
        waves: vec![],
    };
    bank.validate().unwrap();
    Arc::new(bank)
}

/// A track at 150 BPM (a tick a frame) that plays one note of `voice`.
fn note_track(voice: u8, key: u8, gate: u8, then_wait: u8) -> Vec<Command> {
    vec![
        Tempo(75),
        Volume(127),
        Voice(voice),
        Note { gate, key: Some(key), velocity: Some(127) },
        Wait(then_wait),
        Fine,
    ]
}

fn frames(d: &mut Driver, n: usize) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    for _ in 0..n {
        d.step_frame();
        d.take_output(&mut out);
    }
    out
}

fn psg1(d: &Driver) -> Option<ChannelInfo> {
    d.channels().into_iter().find(|c| c.hardware == Hardware::Psg(1))
}

fn loud(samples: &[[f32; 2]]) -> bool {
    samples.iter().any(|s| s[0].abs() > 0.01)
}

#[test]
fn a_note_sounds_through_its_gate_then_releases() {
    let mut d = Driver::new(bank(vec![(0, 50, vec![note_track(SQUARE, 60, 4, 30)])], 4));
    assert!(d.start(SongId(0)));
    let out = frames(&mut d, 1);
    let c = psg1(&d).expect("the note has a channel");
    assert_eq!((c.owner, c.key, c.released), (Some((PlayerId(0), 0)), 60, false));
    assert!(loud(&out));
    frames(&mut d, 3);
    assert!(!psg1(&d).unwrap().released);
    frames(&mut d, 1);
    // Released after four ticks; with no release rate it stops next frame.
    assert!(psg1(&d).is_none_or(|c| c.released));
    let out = frames(&mut d, 2);
    assert!(psg1(&d).is_none() && !loud(&out[out.len() / 2..]));
    assert!(d.player(PlayerId(0)).unwrap().is_playing(), "the track still waits");
    frames(&mut d, 30);
    assert!(!d.player(PlayerId(0)).unwrap().is_playing(), "FINE ends the song");
}

#[test]
fn output_runs_at_32768_hz() {
    let mut d = Driver::new(bank(vec![(0, 50, vec![note_track(SQUARE, 60, 90, 90)])], 4));
    d.start(SongId(0));
    let n = frames(&mut d, 597).len() as f64;
    assert!((n - 32768.0 * 597.0 / m4a::FPS).abs() < 600.0, "{n} samples");
}

#[test]
fn the_fifos_play_each_frames_mix_during_the_next() {
    let mut d = Driver::new(bank(vec![(2, 20, vec![note_track(SAMPLE, 60, 90, 90)])], 4));
    assert_eq!(d.dac_rate(), 65536);
    d.start(SongId(0));
    let mut dac = Vec::new();
    d.step_frame();
    d.take_dac_output(&mut dac);
    // The note's first frame is mixed, but the DAC still plays silence.
    let (right, left) = d.last_mix().unwrap();
    assert!(right.iter().chain(left).any(|&s| s != 0));
    assert!(dac.iter().all(|s| *s == [0, 0]), "the mix sounds in the frame it is mixed");
    let (right, _) = d.last_mix().map(|(r, l)| (r.to_vec(), l.to_vec())).unwrap();
    dac.clear();
    d.step_frame();
    d.take_dac_output(&mut dac);
    // Each byte of it, held a timer period, four DAC steps a level (the
    // FIFOs' latency moves it by a few samples).
    let level = |b: i8| (((b as i32) << 2) * 0x100 * 3 >> 4) as i16;
    let heard: Vec<i16> = dac.iter().map(|s| s[1]).collect();
    let first = right.iter().position(|&b| b != 0).unwrap();
    assert!(heard.contains(&level(right[first])));
    assert_eq!(dac.len(), 1097);
}

#[test]
fn a_busy_effect_player_refuses_lower_priority_songs() {
    let long = vec![note_track(SQUARE, 60, 20, 20)];
    let mut d = Driver::new(bank(vec![(0, 100, long.clone()), (0, 50, long.clone()), (0, 100, long)], 4));
    assert!(d.start(SongId(0)));
    assert!(!d.start(SongId(1)), "lower priority, before the first tick");
    frames(&mut d, 5);
    assert!(!d.start(SongId(1)), "lower priority, while it plays");
    assert!(d.start(SongId(2)), "equal priority replaces");
    assert_eq!(d.player(PlayerId(0)).unwrap().song(), Some(SongId(2)));
    frames(&mut d, 30);
    assert!(d.start(SongId(1)), "anything once it has finished");
}

#[test]
fn the_music_player_takes_any_song() {
    let long = vec![note_track(SQUARE, 60, 20, 20)];
    let mut d = Driver::new(bank(vec![(2, 100, long.clone()), (2, 1, long)], 4));
    assert!(d.start(SongId(0)));
    frames(&mut d, 3);
    assert!(d.start(SongId(1)));
    assert_eq!(d.player(PlayerId(2)).unwrap().song(), Some(SongId(1)));
}

#[test]
fn effects_take_channels_from_the_music_and_give_them_back() {
    let music = vec![
        Tempo(75),
        Volume(100),
        Voice(SQUARE),
        Note { gate: 0, key: Some(48), velocity: Some(100) },
        Wait(40),
        Note { gate: 10, key: Some(50), velocity: None },
        Wait(20),
        Fine,
    ];
    let loud_effect = note_track(SQUARE, 72, 5, 5);
    let quiet_effect = note_track(SQUARE, 84, 5, 5);
    let mut d =
        Driver::new(bank(vec![(2, 20, vec![music]), (0, 128, vec![loud_effect]), (1, 10, vec![quiet_effect])], 4));
    d.start(SongId(0));
    frames(&mut d, 2);
    assert_eq!(psg1(&d).unwrap().owner, Some((PlayerId(2), 0)));
    // A lower-priority effect gets nothing.
    d.start(SongId(2));
    frames(&mut d, 1);
    assert_eq!(psg1(&d).unwrap().owner, Some((PlayerId(2), 0)));
    // A higher one takes the channel; the music's tied note is lost.
    d.start(SongId(1));
    frames(&mut d, 1);
    let c = psg1(&d).unwrap();
    assert_eq!((c.owner, c.key), (Some((PlayerId(0), 0)), 72));
    frames(&mut d, 10);
    assert!(psg1(&d).is_none(), "the effect ended; the music's note doesn't come back");
    // The music's next note finds the channel free.
    frames(&mut d, 30);
    let c = psg1(&d).unwrap();
    assert_eq!((c.owner, c.key), (Some((PlayerId(2), 0)), 50));
}

#[test]
fn equal_priorities_favor_the_earlier_track() {
    // Both tracks want square 1 on the same tick: track 0 plays first and
    // keeps it (track 1's note yields to a track that sorts first).
    let t0 = note_track(SQUARE, 60, 30, 30);
    let t1 = vec![Voice(SQUARE), Volume(127), Note { gate: 30, key: Some(67), velocity: Some(127) }, Wait(30), Fine];
    let mut d = Driver::new(bank(vec![(2, 20, vec![t0.clone(), t1.clone()])], 4));
    d.start(SongId(0));
    frames(&mut d, 1);
    assert_eq!(psg1(&d).unwrap().owner, Some((PlayerId(2), 0)));
    // Track 1 first, track 0 a tick later: track 0 takes the channel.
    let late0 = [vec![Wait(1)], t0].concat();
    let mut d = Driver::new(bank(vec![(2, 20, vec![late0, t1])], 4));
    d.start(SongId(0));
    frames(&mut d, 1);
    assert_eq!(psg1(&d).unwrap().owner, Some((PlayerId(2), 1)));
    frames(&mut d, 1);
    assert_eq!(psg1(&d).unwrap().owner, Some((PlayerId(2), 0)));
}

#[test]
fn direct_sound_notes_steal_the_weakest_channel() {
    let held = |key| {
        vec![
            Tempo(75),
            Volume(127),
            Voice(SAMPLE),
            Note { gate: 0, key: Some(key), velocity: Some(127) },
            Wait(60),
            Fine,
        ]
    };
    let mut d = Driver::new(bank(
        vec![(2, 20, vec![held(60), held(62)]), (0, 128, vec![held(72)]), (1, 10, vec![held(74)])],
        2,
    ));
    d.start(SongId(0));
    frames(&mut d, 1);
    let owners = |d: &Driver| {
        d.channels()
            .iter()
            .filter(|c| matches!(c.hardware, Hardware::DirectSound(_)))
            .map(|c| c.owner)
            .collect::<Vec<_>>()
    };
    assert_eq!(owners(&d), [Some((PlayerId(2), 0)), Some((PlayerId(2), 1))]);
    d.start(SongId(2));
    frames(&mut d, 1);
    assert_eq!(owners(&d), [Some((PlayerId(2), 0)), Some((PlayerId(2), 1))], "too weak");
    d.start(SongId(1));
    frames(&mut d, 1);
    // Of equal priorities, the later track's note goes.
    assert_eq!(owners(&d), [Some((PlayerId(2), 0)), Some((PlayerId(0), 0))]);
}

#[test]
fn a_player_plays_only_as_many_tracks_as_it_has() {
    let t = |key| {
        vec![
            Tempo(75),
            Volume(127),
            Voice(SAMPLE),
            Note { gate: 10, key: Some(key), velocity: Some(127) },
            Wait(10),
            Fine,
        ]
    };
    let mut d = Driver::new(bank(vec![(0, 50, vec![t(60), t(62), t(64)])], 4));
    d.start(SongId(0));
    frames(&mut d, 1);
    assert_eq!(d.channels().len(), 2);
}

#[test]
fn tempo_control_scales_the_tick_rate() {
    let mut d = Driver::new(bank(vec![(2, 20, vec![note_track(SQUARE, 60, 200, 200)])], 4));
    d.start(SongId(0));
    frames(&mut d, 10);
    assert_eq!(d.player(PlayerId(2)).unwrap().clock(), 10);
    d.set_tempo(PlayerId(2), 0x200);
    frames(&mut d, 10);
    assert_eq!(d.player(PlayerId(2)).unwrap().clock(), 30);
    // EXE6's pinch tempo: 150 * 0x11A / 0x100 = 165 per frame.
    d.set_tempo(PlayerId(2), 0x11A);
    assert_eq!(d.player(PlayerId(2)).unwrap().tempo_step(), 165);
}

#[test]
fn pitch_control_shifts_sounding_notes() {
    let mut d = Driver::new(bank(vec![(2, 20, vec![note_track(SQUARE, 60, 200, 200)])], 4));
    d.start(SongId(0));
    frames(&mut d, 2);
    d.set_pitch(PlayerId(2), 0xFFFF, 0x100);
    frames(&mut d, 1);
    let freq = |key| m4a::tables::midi_key_to_cgb_freq(1, key, 0);
    assert_eq!(psg1(&d).unwrap().frequency, freq(61));
    d.set_pitch(PlayerId(2), 0xFFFF, -0x200);
    frames(&mut d, 1);
    assert_eq!(psg1(&d).unwrap().frequency, freq(58));
}

#[test]
fn a_starting_song_clears_earlier_controls() {
    // Tempo control survives a song start; volume and pitch reset with the tracks.
    let mut d = Driver::new(bank(vec![(2, 20, vec![note_track(SQUARE, 60, 200, 200)])], 4));
    d.start(SongId(0));
    d.set_pitch(PlayerId(2), 0xFFFF, 0x100);
    d.set_tempo(PlayerId(2), 0x200);
    frames(&mut d, 1);
    assert_eq!(psg1(&d).unwrap().frequency, m4a::tables::midi_key_to_cgb_freq(1, 60, 0));
    assert_eq!(d.player(PlayerId(2)).unwrap().tempo_step(), 300);
}

#[test]
fn volume_control_scales_the_note() {
    let mut d = Driver::new(bank(vec![(2, 20, vec![note_track(SQUARE, 60, 200, 200)])], 4));
    d.start(SongId(0));
    frames(&mut d, 1);
    let full = psg1(&d).unwrap().volume;
    d.set_volume(PlayerId(2), 0xFFFF, 0x80);
    frames(&mut d, 1);
    let half = psg1(&d).unwrap().volume;
    assert!(half.0 < full.0 && half.0 >= full.0 / 2 - 1, "{full:?} {half:?}");
}

#[test]
fn stopping_ends_notes_at_once() {
    let mut d = Driver::new(bank(
        vec![(2, 20, vec![note_track(SAMPLE, 60, 200, 200)]), (0, 50, vec![note_track(SQUARE, 60, 200, 200)])],
        4,
    ));
    d.start(SongId(0));
    d.start(SongId(1));
    frames(&mut d, 2);
    assert_eq!(d.channels().len(), 2);
    d.stop_all();
    let out = frames(&mut d, 2);
    assert!(d.channels().is_empty());
    // (The hardware plays a frame behind the mix.)
    assert!(!loud(&out[out.len() * 3 / 4..]));
    assert!(!d.player(PlayerId(2)).unwrap().is_playing());
}

#[test]
fn a_fade_out_stops_the_player() {
    let mut d = Driver::new(bank(vec![(2, 20, vec![note_track(SQUARE, 60, 250, 250)])], 4));
    d.start(SongId(0));
    frames(&mut d, 2);
    d.fade_out(PlayerId(2), 2);
    frames(&mut d, 16 * 2 - 1);
    assert!(d.player(PlayerId(2)).unwrap().is_playing());
    frames(&mut d, 1);
    assert!(!d.player(PlayerId(2)).unwrap().is_playing());
    assert!(d.channels().is_empty());
}

#[test]
fn ties_hold_until_their_end() {
    let t = vec![
        Tempo(75),
        Volume(127),
        Voice(SQUARE),
        Note { gate: 0, key: Some(64), velocity: Some(127) },
        Wait(96),
        Wait(20),
        EndTie { key: None },
        Wait(10),
        Fine,
    ];
    let mut d = Driver::new(bank(vec![(2, 20, vec![t])], 4));
    d.start(SongId(0));
    frames(&mut d, 116);
    assert!(!psg1(&d).unwrap().released);
    frames(&mut d, 1);
    assert!(psg1(&d).is_none_or(|c| c.released));
}

#[test]
fn patterns_repeats_and_loops() {
    // [0] a note; [1] wait; [2] repeat [0] three times; [3] call the
    // pattern at [6]; [4] loop back to [3]; [6..] a note and PEND.
    let t = vec![
        Note { gate: 1, key: Some(60), velocity: Some(127) },
        Wait(2),
        Repeat { count: 3, target: 0 },
        Call(6),
        Goto(3),
        Fine,
        Note { gate: 1, key: Some(72), velocity: None },
        Wait(4),
        Return,
    ];
    let t = [vec![Tempo(75), Volume(127), Voice(SQUARE)], t.into_iter().map(|c| shift(c, 3)).collect()].concat();
    let mut d = Driver::new(bank(vec![(2, 20, vec![t])], 4));
    d.start(SongId(0));
    let mut keys = Vec::new();
    for _ in 0..30 {
        frames(&mut d, 1);
        if let Some(c) = psg1(&d).filter(|c| !c.released) {
            keys.push(c.key);
        }
    }
    // Three notes two ticks apart, then the pattern's every four ticks.
    assert_eq!(keys[..3], [60, 60, 60]);
    assert!(keys[3..].iter().all(|&k| k == 72) && keys.len() >= 8, "{keys:?}");
    assert!(d.player(PlayerId(2)).unwrap().is_playing(), "loops forever");
}

#[test]
fn memacc_writes_the_memory_area_and_jumps_on_it() {
    let jump = MemOp::JumpIf { test: MemTest::Equal, with_memory: false, target: 7 };
    let track = vec![
        Tempo(75),
        Volume(127),
        Voice(SQUARE),
        MemAcc { op: MemOp::Set, address: 3, operand: 9 },
        MemAcc { op: jump, address: 3, operand: 9 },
        Note { gate: 4, key: Some(60), velocity: Some(127) },
        Fine,
        Note { gate: 4, key: Some(72), velocity: Some(127) },
        Wait(8),
        MemAcc { op: MemOp::AddFromMemory, address: 3, operand: 4 },
        Fine,
    ];
    let mut d = Driver::new(bank(vec![(2, 20, vec![track])], 4));
    d.set_memory(4, 2);
    d.start(SongId(0));
    frames(&mut d, 1);
    assert_eq!(d.memory()[3], 9);
    assert_eq!(psg1(&d).unwrap().midi_key, 72, "the jump skipped key 60");
    frames(&mut d, 10);
    assert_eq!(d.memory()[3], 11, "plus the game's byte");
}

/// A command with its jump targets moved by `n`.
fn shift(c: Command, n: u32) -> Command {
    match c {
        Goto(i) => Goto(i + n),
        Call(i) => Call(i + n),
        Repeat { count, target } => Repeat { count, target: target + n },
        c => c,
    }
}
