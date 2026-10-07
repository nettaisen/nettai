use super::*;
use m4a::bank::*;

/// Two of the game's songs, as a battle's cues name them: its netbattle
/// music and its victory music.
const VIRUS_BATTLE: SoundId = SoundId(0x15);
const WINNER: SoundId = SoundId(0x1F);

/// EXE6's player layout with two songs: the battle music (0x15) on the
/// music player and an effect (0x94) on player 16.
fn bank() -> Arc<SoundBank> {
    let square = Voice {
        kind: VoiceKind::Square1 { duty: 2, sweep: m4a::bank::NO_SWEEP, fixed: false },
        key: 60,
        pan: None,
        length: 0,
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
        mixer: m4a::bank::MixerConfig { mix_rate: 10512, ds_channels: 4, master_volume: 15, reverb: 0, dac_resolution: 1 },
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
    assert_eq!(requests(&mut c, SoundCue::Music(WINNER)), start(0x1F));
    assert_eq!(requests(&mut c, SoundCue::StopMusic), [Request::StopAll]);
    assert_eq!(requests(&mut c, SoundCue::Music(WINNER)), start(0x1F), "stopping forgets the music");
    assert_eq!(requests(&mut c, SoundCue::Music(NO_MUSIC)), [Request::StopAll]);
    assert_eq!(requests(&mut c, SoundCue::Music(NO_MUSIC)), []);
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
        requests(&mut c, SoundCue::Volume { player: 22, volume: 0x80 }),
        [Request::Volume { player: PlayerId(22), tracks: 0xFFFF, volume: 0x80 }]
    );
}

#[test]
fn canceling_a_cue_takes_it_back() {
    let mut c = SoundCalls::new();
    let music = |id| SoundCue::Music(SoundId(id));
    requests(&mut c, music(0x15));
    requests(&mut c, SoundCue::Music(WINNER));
    let mut out = Vec::new();
    c.cancel(SoundCue::Music(WINNER), &mut out);
    assert_eq!(out, [Request::Start(SongId(0x15))], "back to the battle music");
    assert_eq!(requests(&mut c, music(0x15)), [], "which is the music again");
    requests(&mut c, SoundCue::StopMusic);
    let mut out = Vec::new();
    c.cancel(SoundCue::StopMusic, &mut out);
    assert_eq!(out, [Request::Start(SongId(0x15))]);
    let mut out = Vec::new();
    c.cancel(SoundCue::Effect(SoundId(0x94)), &mut out);
    c.cancel(SoundCue::Pinch(true), &mut out);
    let m = MUSIC_PLAYER;
    assert_eq!(
        out,
        [
            Request::Stop(SongId(0x94)),
            Request::Pitch { player: m, tracks: 0xFFFF, pitch: 0 },
            Request::Tempo { player: m, tempo: 0x100 }
        ]
    );
}

#[test]
fn a_canceled_effect_stops() {
    let mut a = BattleAudio::new(bank(), Songs::numbers(0x200));
    let mut out = Vec::new();
    a.handle_actions([CueAction::Play(SoundCue::Effect(SoundId(0x94)))]);
    a.tick(&mut out);
    assert_eq!(a.driver().player(PlayerId(16)).unwrap().song(), Some(SongId(0x94)));
    assert!(a.driver().player(PlayerId(16)).unwrap().is_playing());
    a.handle_actions([CueAction::Cancel(SoundCue::Effect(SoundId(0x94)))]);
    a.tick(&mut out);
    assert!(!a.driver().player(PlayerId(16)).unwrap().is_playing());
}

#[test]
fn a_cue_sounds_two_frames_later_as_in_the_game() {
    let mut a = BattleAudio::new(bank(), Songs::numbers(0x200));
    let mut out = Vec::new();
    a.handle(&[SoundCue::Music(VIRUS_BATTLE)]);
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
    let mut a = BattleAudio::new(bank(), Songs::numbers(0x200));
    let mut out = Vec::new();
    a.handle(&[SoundCue::Music(VIRUS_BATTLE)]);
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
    let mut a = BattleAudio::new(bank(), Songs::numbers(0x200));
    a.handle(&[SoundCue::Effect(SoundId(0x94)); 40]);
    assert_eq!(a.packs[0].queue.len(), QUEUE_LIMIT);
    a.tick(&mut Vec::new());
    assert!(a.packs[0].queue.is_empty());
}

#[test]
fn a_battle_drives_the_music() {
    use nettai_battle::content::testing;
    use nettai_battle::{Battle, PlayerTick, TickEvents};
    let content = testing::restaged(testing::LINK_BATTLE_SIDE0_FIRST, |s| s.music = Some(nettai_battle::SoundId(0)));
    let mut setup = testing::round_setup(testing::LINK_BATTLE_SIDE0_FIRST, testing::stats(500));
    testing::on(&mut setup, &content);
    let songs = Songs::of(&content.assets);
    let mut b = Battle::new(setup, std::sync::Arc::new(content));
    let mut a = BattleAudio::new(bank(), songs);
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

/// docs/design/rules-in-luau.md §7.4: each pack's songs play on its own
/// driver, with its own instruments; music of another pack stops the
/// music that played, and the drivers' outputs are added.
#[test]
fn each_pack_plays_its_own_songs() {
    use nettai_battle::content::{InPack, PackId};
    let songs = Songs(vec![InPack { pack: PackId(0), id: VIRUS_BATTLE.0 }, InPack { pack: PackId(1), id: VIRUS_BATTLE.0 }]);
    let mut a = BattleAudio::with_banks(vec![bank(), bank()], songs);
    let mut out = Vec::new();
    let playing = |a: &BattleAudio, p: u8| a.driver_of(PackId(p)).unwrap().player(MUSIC_PLAYER).is_some_and(|m| m.is_playing());
    a.handle(&[SoundCue::Music(SoundId(0))]);
    a.tick(&mut out);
    a.tick(&mut out);
    assert!(playing(&a, 0) && !playing(&a, 1));
    a.handle(&[SoundCue::Music(SoundId(1))]);
    a.tick(&mut out);
    a.tick(&mut out);
    assert!(!playing(&a, 0) && playing(&a, 1), "the music moved to the other pack's player");
    // One frame's samples, however many packs.
    out.clear();
    a.tick(&mut out);
    assert!((out.len() as i64 - 549).abs() <= 8, "{} samples for a frame", out.len());
}
