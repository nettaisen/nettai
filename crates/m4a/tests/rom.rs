//! Reading sound data out of a small ROM image built here: the driver's
//! tables, song headers, track bytes (running status, patterns, loops, ties)
//! and voicegroups, and the bank file round trip.

use m4a::bank::*;
use m4a::rom::{RomError, extract};

const BASE: u32 = 0x0800_0000;

struct Image(Vec<u8>);

impl Image {
    fn put(&mut self, at: u32, bytes: &[u8]) {
        let o = (at - BASE) as usize;
        self.0[o..o + bytes.len()].copy_from_slice(bytes);
    }
    fn word(&mut self, at: u32, v: u32) {
        self.put(at, &v.to_le_bytes());
    }
}

const VOICEGROUP: u32 = BASE + 0x400;
const SAMPLE: u32 = BASE + 0xB00;
const KEY_MAP: u32 = BASE + 0xC00;
const TRACK0: u32 = BASE + 0xE00;
const PATTERN: u32 = BASE + 0xE40;
const TRACK1: u32 = BASE + 0xE80;
const BAD_TRACK: u32 = BASE + 0xF00;

fn image() -> Image {
    let mut rom = Image(vec![0; 0x1000]);
    rom.put(BASE + 0xA0, b"TESTROM\0\0\0\0\0TEST");
    // The boot sound mode (10512 Hz, 4 channels, volume 15), then
    // m4aSongNumStart and its literal pool.
    rom.word(BASE + 0x100, 0x0093_F400);
    let at = BASE + 0x104;
    rom.put(at, &[0x00, 0xB5, 0x00, 0x04, 0x07, 0x4A, 0x08, 0x49, 0x40, 0x0B, 0x40, 0x18]);
    rom.word(at + 36, BASE + 0x200);
    rom.word(at + 40, BASE + 0x300);
    // Players: 0 has its tracks after 1's in memory; 1 uses priorities.
    for (p, (tracks, max, priority)) in [(0x0200_0100u32, 2u8, 0u16), (0x0200_0000, 4, 1)].into_iter().enumerate() {
        let e = BASE + 0x200 + 12 * p as u32;
        rom.word(e, 0x0200_1000);
        rom.word(e + 4, tracks);
        rom.put(e + 8, &[max, 0]);
        rom.put(e + 10, &priority.to_le_bytes());
    }
    // Songs: 0 on player 1; 1 empty; 2 on player 0, unreadable; then the end.
    let song = |rom: &mut Image, i: u32, header: u32, player: u16| {
        rom.word(BASE + 0x300 + 8 * i, header);
        rom.put(BASE + 0x300 + 8 * i + 4, &player.to_le_bytes());
    };
    song(&mut rom, 0, BASE + 0xD00, 1);
    song(&mut rom, 1, BASE + 0xD20, 0);
    song(&mut rom, 2, BASE + 0xD40, 0);
    rom.put(BASE + 0xD00, &[2, 0, 30, 0x85]);
    rom.word(BASE + 0xD04, VOICEGROUP);
    rom.word(BASE + 0xD08, TRACK0);
    rom.word(BASE + 0xD0C, TRACK1);
    rom.put(BASE + 0xD20, &[0, 0, 0, 0]);
    rom.word(BASE + 0xD24, VOICEGROUP);
    rom.put(BASE + 0xD40, &[1, 0, 10, 0]);
    rom.word(BASE + 0xD44, VOICEGROUP);
    rom.word(BASE + 0xD48, BAD_TRACK);
    // Voices: a sample, a square, a drum kit (this voicegroup again), a
    // key split (its first two voices), and nothing past them.
    rom.put(VOICEGROUP, &[0x00, 60, 0, 0]);
    rom.word(VOICEGROUP + 4, SAMPLE);
    rom.put(VOICEGROUP + 8, &[255, 0, 255, 200]);
    rom.put(VOICEGROUP + 12, &[0x02, 60, 0, 0, 1, 0, 0, 0, 0, 0, 13, 2]);
    rom.put(VOICEGROUP + 24, &[0x80, 0, 0, 0]);
    rom.word(VOICEGROUP + 28, VOICEGROUP);
    rom.put(VOICEGROUP + 36, &[0x40, 0, 0, 0]);
    rom.word(VOICEGROUP + 40, VOICEGROUP);
    rom.word(VOICEGROUP + 44, KEY_MAP);
    rom.put(KEY_MAP + 64, &[1; 64]);
    // A looping 16-byte sample at 10512 Hz.
    rom.put(SAMPLE + 2, &0x4000u16.to_le_bytes());
    rom.word(SAMPLE + 4, 10512 * 1024);
    rom.word(SAMPLE + 8, 4);
    rom.word(SAMPLE + 12, 16);
    rom.put(SAMPLE + 16, &[0, 20, 40, 60, 80, 60, 40, 20, 0, 0xEC, 0xD8, 0xC4, 0xB0, 0xC4, 0xD8, 0xEC]);
    // Track 0: TEMPO 75, VOICE 1, VOL 100, then 0x60 by running status;
    // N09 60 127, W24, 62 (running status: a note, key only), W24, PATT,
    // FINE. The pattern: TIE 64 100, W12, EOT 64, PEND, FINE.
    rom.put(TRACK0, &[0xBB, 75, 0xBD, 1, 0xBE, 100, 0x60, 0xD8, 60, 127, 0x98, 62, 0x98, 0xB3]);
    rom.word(TRACK0 + 14, PATTERN);
    rom.put(TRACK0 + 18, &[0xB1]);
    rom.put(PATTERN, &[0xCF, 64, 100, 0x8C, 0xCE, 64, 0xB4, 0xB1]);
    // Track 1: VOICE 0, then a loop of N04 72 96 with 5 extra ticks, W04.
    rom.put(TRACK1, &[0xBD, 0, 0xD3, 72, 96, 5, 0x84, 0xB2]);
    rom.word(TRACK1 + 8, TRACK1 + 2);
    // The unreadable track loops back to a data byte: which command it
    // repeats would depend on the path taken.
    rom.put(BAD_TRACK, &[0xBE, 100, 0x50, 0xB2]);
    rom.word(BAD_TRACK + 4, BAD_TRACK + 2);
    rom
}

#[test]
fn a_rom_reads_into_a_bank() {
    let (bank, failures) = extract(&image().0).unwrap();
    assert_eq!(bank.mixer, MixerConfig { mix_rate: 10512, ds_channels: 4, master_volume: 15, reverb: 0 });
    assert_eq!(
        bank.players,
        [
            PlayerConfig { max_tracks: 2, uses_priority: false, track_order: 1 },
            PlayerConfig { max_tracks: 4, uses_priority: true, track_order: 0 },
        ]
    );
    assert_eq!(failures, [(SongId(2), RomError::RunningStatus(BAD_TRACK + 2))]);
    assert_eq!(bank.songs.len(), 3);
    assert!(bank.songs[1].is_none() && bank.songs[2].is_none());
    let song = bank.song(SongId(0)).unwrap();
    assert_eq!((song.player, song.priority, song.reverb), (PlayerId(1), 30, Some(5)));
    use Command::*;
    assert_eq!(
        song.tracks[0].commands,
        [
            Tempo(75),
            Command::Voice(1),
            Volume(100),
            Volume(0x60),
            Note { gate: 9, key: Some(60), velocity: Some(127) },
            Wait(24),
            Note { gate: 9, key: Some(62), velocity: None },
            Wait(24),
            Call(10),
            Fine,
            Note { gate: 0, key: Some(64), velocity: Some(100) },
            Wait(12),
            EndTie { key: Some(64) },
            Return,
            Fine,
        ]
    );
    assert_eq!(
        song.tracks[1].commands,
        [Command::Voice(0), Note { gate: 4 + 5, key: Some(72), velocity: Some(96) }, Wait(4), Goto(1)]
    );
    let voices = &bank.voicegroups[song.voicegroup.0 as usize].voices;
    assert_eq!(voices.len(), 128);
    let sample = SampleId(0);
    assert_eq!(voices[0].kind, VoiceKind::DirectSound { sample, fixed: false });
    assert_eq!(voices[0].envelope, Envelope { attack: 255, decay: 0, sustain: 255, release: 200 });
    assert_eq!(voices[1].kind, VoiceKind::Square2 { duty: 1 });
    let VoiceKind::Drums { kit } = voices[2].kind else { panic!("{:?}", voices[2]) };
    let kit = &bank.voicegroups[kit.0 as usize].voices;
    assert_eq!(kit[1].kind, VoiceKind::Square2 { duty: 1 });
    assert_eq!(kit[2].kind, VoiceKind::Silent, "a kit in a kit plays nothing");
    let VoiceKind::Split { group, map } = voices[3].kind else { panic!("{:?}", voices[3]) };
    assert_eq!(bank.voicegroups[group.0 as usize].voices.len(), 2);
    assert_eq!((bank.key_maps[map.0 as usize].0[63], bank.key_maps[map.0 as usize].0[64]), (0, 1));
    assert!(voices[4..].iter().all(|v| v.kind == VoiceKind::Silent));
    let s = &bank.samples[0];
    assert_eq!((s.rate, s.loop_start, s.data.len(), s.data[9]), (10512 * 1024, Some(4), 16, -20));
}

#[test]
fn a_bank_survives_its_file_format() {
    let (bank, _) = extract(&image().0).unwrap();
    let bytes = bank.to_bytes();
    assert_eq!(SoundBank::from_bytes(&bytes), Ok(bank));
    assert_eq!(SoundBank::from_bytes(b"RIFF...."), Err(BankError::NotABank));
    assert_eq!(SoundBank::from_bytes(&bytes[..bytes.len() - 1]), Err(BankError::Truncated));
    let mut bad = bytes.clone();
    bad[8] = 99;
    assert_eq!(SoundBank::from_bytes(&bad), Err(BankError::Version(99)));
}

#[test]
fn a_bank_with_a_dangling_reference_is_refused() {
    let (mut bank, _) = extract(&image().0).unwrap();
    bank.songs[0].as_mut().unwrap().tracks[1].commands[3] = Command::Goto(99);
    assert_eq!(SoundBank::from_bytes(&bank.to_bytes()), Err(BankError::Invalid("jump")));
}

#[test]
fn not_a_rom() {
    assert_eq!(extract(&[0; 16]).err(), Some(RomError::TooSmall));
    assert_eq!(extract(&[0; 0x1000]).err(), Some(RomError::NoDriver));
}
