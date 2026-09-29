use super::*;
use m4a::bank::*;

/// BN6's player layout with two songs: the battle music (0x15) on the
/// music player and an effect (0x94) on player 16.
fn bank() -> Arc<SoundBank> {
    let square = Voice {
        kind: VoiceKind::Square1 { duty: 2 },
        key: 60,
        pan: None,
        envelope: Envelope { sustain: 15, ..Envelope::default() },
    };
    let track = |key| Track {
        commands: vec![
            Command::Tempo(75),
            Command::Volume(100),
            Command::Voice(0),
            Command::Note { gate: 0, key: Some(key), velocity: Some(100) },
            Command::Wait(96),
            Command::Goto(4),
        ],
    };
    let song = |player: u8, priority, key| {
        Some(Song {
            player: PlayerId(player),
            priority,
            reverb: None,
            voicegroup: VoicegroupId(0),
            tracks: vec![track(key)],
        })
    };
    let mut songs = vec![None; 0x95];
    songs[0x15] = song(31, 20, 48);
    songs[0x1F] = song(31, 20, 50);
    songs[0x94] = song(16, 128, 72);
    let players = (0..32)
        .map(|p| PlayerConfig { max_tracks: if p == 31 { 8 } else { 2 }, uses_priority: p != 31, track_order: p as u8 })
        .collect();
    Arc::new(SoundBank {
        mixer: m4a::bank::MixerConfig { mix_rate: 10512, ds_channels: 4, master_volume: 15, reverb: 0 },
        players,
        songs,
        voicegroups: vec![Voicegroup { voices: vec![square] }],
        key_maps: vec![],
        samples: vec![],
        waves: vec![],
    })
}

fn requests(calls: &mut SoundCalls, cue: SoundCue) -> Vec<Request> {
    let mut out = Vec::new();
    calls.requests(cue, &mut out);
    out
}

#[test]
fn play_music_skips_the_music_already_playing() {
    let mut c = SoundCalls::new();
    let start = |id| vec![Request::Start(SongId(id))];
    assert_eq!(requests(&mut c, SoundCue::Music(SoundId(0x15))), start(0x15));
    assert_eq!(requests(&mut c, SoundCue::Music(SoundId(0x15))), []);
    assert_eq!(requests(&mut c, SoundCue::Music(SoundId::WINNER)), start(0x1F));
    assert_eq!(requests(&mut c, SoundCue::StopMusic), [Request::StopAll]);
    assert_eq!(requests(&mut c, SoundCue::Music(SoundId::WINNER)), start(0x1F), "stopping forgets the music");
    assert_eq!(requests(&mut c, SoundCue::Music(SoundId::NO_MUSIC)), [Request::StopAll]);
    assert_eq!(requests(&mut c, SoundCue::Music(SoundId::NO_MUSIC)), []);
    assert_eq!(requests(&mut c, SoundCue::Effect(SoundId(0x94))), start(0x94));
    assert_eq!(requests(&mut c, SoundCue::Effect(SoundId(0x94))), start(0x94), "effects always go");
}

#[test]
fn pinch_and_volume_cues_are_player_controls() {
    let mut c = SoundCalls::new();
    let m = MUSIC_PLAYER;
    assert_eq!(
        requests(&mut c, SoundCue::Pinch(true)),
        [Request::Pitch { player: m, tracks: 0xFFFF, pitch: 0x100 }, Request::Tempo { player: m, tempo: 0x11A }]
    );
    assert_eq!(
        requests(&mut c, SoundCue::Pinch(false)),
        [Request::Pitch { player: m, tracks: 0xFFFF, pitch: 0 }, Request::Tempo { player: m, tempo: 0x100 }]
    );
    assert_eq!(
        requests(&mut c, SoundCue::RestoreVolume),
        [
            Request::Volume { player: m, tracks: 0xFFFF, volume: 0x100 },
            Request::Volume { player: CUSTOM_SCREEN_PLAYER, tracks: 0xFFFF, volume: 0x100 }
        ]
    );
}

#[test]
fn a_cue_sounds_two_frames_later_as_in_the_game() {
    let mut a = BattleAudio::new(bank());
    let mut out = Vec::new();
    a.handle(&[SoundCue::Music(SoundId::VIRUS_BATTLE)]);
    // The frame's VBlank comes first; the queued call runs after it.
    a.tick(&mut out);
    assert_eq!(a.driver().player(MUSIC_PLAYER).unwrap().song(), Some(SongId(0x15)));
    assert!(a.driver().channels().is_empty());
    a.tick(&mut out);
    assert_eq!(a.driver().channels().len(), 1);
    // (A few samples wait for the next frame's Direct Sound mix.)
    assert!((out.len() as i64 - 2 * 549).abs() <= 8, "{} samples for two frames", out.len());
}

#[test]
fn effects_play_over_the_music() {
    let mut a = BattleAudio::new(bank());
    let mut out = Vec::new();
    a.handle(&[SoundCue::Music(SoundId::VIRUS_BATTLE)]);
    for _ in 0..3 {
        a.tick(&mut out);
    }
    a.handle(&[SoundCue::Effect(SoundId(0x94))]);
    a.tick(&mut out);
    a.tick(&mut out);
    let owners: Vec<_> = a.driver().channels().iter().map(|c| c.owner.map(|o| o.0)).collect();
    assert_eq!(owners, [Some(PlayerId(16))], "the effect took the music's square channel");
}

#[test]
fn the_game_queue_holds_32_calls_a_frame() {
    let mut a = BattleAudio::new(bank());
    a.handle(&[SoundCue::Effect(SoundId(0x94)); 40]);
    assert_eq!(a.queue.len(), QUEUE_LIMIT);
    a.tick(&mut Vec::new());
    assert!(a.queue.is_empty());
}

#[test]
fn a_battle_drives_the_music() {
    use bn6_battle::setup::{BattleSettings, NaviStats, RoundSetup, SetScore, effects};
    use bn6_battle::{Battle, PlayerTick, TickEvents};
    let stats = NaviStats { hp: 500, max_hp: 500, max_base_hp: 500, ..NaviStats::default() };
    let mut b = Battle::new(RoundSetup {
        settings: BattleSettings {
            layout: 0,
            unk_01: 0,
            music: 0,
            mode: 0,
            background: 0,
            battle_number: 0,
            panel_pattern: 0x38,
            unk_07: 0,
            effects: effects::LINK,
            actors: &bn6_battle::data::ACTOR_LISTS[0],
        },
        navi_stats: [stats; 2],
        rng: 1,
        local_side: 0,
        score: SetScore::default(),
    });
    let mut a = BattleAudio::new(bank());
    let mut out = Vec::new();
    for _ in 0..4 {
        b.tick(&[PlayerTick::default(), PlayerTick::default()], TickEvents::default());
        a.handle(b.sound_cues());
        a.tick(&mut out);
    }
    let music = a.driver().player(MUSIC_PLAYER).unwrap();
    assert_eq!(music.song(), Some(SongId(0x15)));
    assert!(music.is_playing());
    // The first tick's pinch switch was undone on the second.
    assert_eq!(music.tempo_step(), 150);
}

#[test]
fn wav_files_are_16_bit_stereo() {
    let b = wav::to_bytes(&[[0.5, -0.5], [2.0, 0.0]], 32768);
    assert_eq!(b.len(), 44 + 8);
    assert_eq!(&b[0..4], b"RIFF");
    assert_eq!(u32::from_le_bytes(b[24..28].try_into().unwrap()), 32768);
    assert_eq!(u16::from_le_bytes(b[22..24].try_into().unwrap()), 2);
    let s = |i: usize| i16::from_le_bytes(b[44 + 2 * i..46 + 2 * i].try_into().unwrap());
    assert_eq!((s(0), s(1), s(2), s(3)), (16384, -16384, 32767, 0));
}
