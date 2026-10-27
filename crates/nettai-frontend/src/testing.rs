//! What tests of a set played back are made from, for this crate's tests
//! and those of the crates over it (nettai-video's; the `testing`
//! feature): a short set recorded on content the caller brings (the
//! repository's test content), graphics that draw a field without a pack,
//! and a sound bank that makes a sound of every cue. Nothing here comes
//! from a ROM.

use crate::driver::LivePlayer;
pub use crate::driver::short_set;
use crate::game::Graphics;
use crate::replay::{Info, MatchKind, Recorder, Replay};
use crate::session::Session;
use nettai_audio::BattleAudio;
use nettai_battle::content::Content;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// A sink a test writes a replay to and reads it back from.
#[derive(Clone, Default)]
pub struct Shared(pub Arc<Mutex<Vec<u8>>>);

impl Shared {
    pub fn bytes(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

impl Write for Shared {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A short set of `game` on `content` from `seed`, played live and
/// recorded (`short_set::of`: the right navi has 1 HP, the left player
/// shoots and wins both rounds): the replay as its file reads back, and
/// the set's ticks.
pub fn recorded(content: &Arc<Content>, game: &str, seed: u32) -> (Replay, u64) {
    recorded_with(content, game, seed, |_, _| 0)
}

/// [`recorded`], the left player holding `also(battle, tick)` besides the
/// shooter's buttons (a START in the fight, which shoots no less).
pub fn recorded_with(content: &Arc<Content>, game: &str, seed: u32, mut also: impl FnMut(&nettai_battle::Battle, u32) -> u16) -> (Replay, u64) {
    let m = short_set::of(content, game, seed);
    let sink = Shared::default();
    let mut s = Session::new(Box::new(LivePlayer::new(nettai_match::Set::of(content, &m, seed))));
    let info = Info { when: 1_790_000_000, names: ["Lan".into(), String::new()], ..Info::of(MatchKind::Offline, 0) };
    s.record(Recorder::new(Box::new(sink.clone()), content, &m, &info).unwrap()).unwrap();
    loop {
        let tick = s.ticks as u32;
        let keys = short_set::shooter(&s.battle, 0, tick) | also(&s.battle, tick);
        if !s.step(keys) {
            break;
        }
    }
    assert_eq!(s.result, Some(nettai_battle::BattleResult::Won), "{game}");
    (Replay::read(&sink.bytes()).unwrap(), s.ticks)
}

/// Graphics whose panels are one solid tile of `color` (the field's alone:
/// no sprite, no lettering), for `content`'s panel types.
pub fn graphics(content: &Content, color: u16) -> Graphics {
    use nettai_assets::{Bundle, Field, MapEntry, Tiles};
    let block = [MapEntry { tile: 0xA3, hflip: false, vflip: false, palette: 1 }; 15];
    let mut palettes = vec![[0u16; 16]; 8];
    palettes[0][1] = color;
    let panel_types: Vec<String> = content.rules().panels.names.iter().take(13).cloned().collect();
    let field = Field {
        tiles: Tiles { pixels: vec![1; Tiles::TILE] },
        first_tile: 0xA3,
        palettes,
        first_palette: 1,
        panels: (0..panel_types.len() * 6).map(|_| block).collect(),
        panel_types,
        front_edges: [[MapEntry::default(); 5]; 2],
        highlights: vec![[MapEntry::default(); 15]; 2],
        ..Field::default()
    };
    Graphics {
        bundles: vec![Arc::new(Bundle { field, ..Bundle::default() })],
        others: Default::default(),
        strings: None,
        own: nettai_battle::content::PackId(0),
        assets: Arc::new(numbered_by_handle(content)),
        report: Default::default(),
    }
}

/// A made-up pack's numbers for `content`'s assets: each its handle (as
/// [`audio`]'s songs are).
pub fn numbered_by_handle(content: &Content) -> nettai_battle::content::AssetNumbers {
    let a = &content.assets;
    let small = |n: usize| (0..n).map(|h| h as u8).collect();
    nettai_battle::content::AssetNumbers {
        sprites: (0..a.sprites.len()).map(|h| nettai_battle::content::SpriteNumber { category: (h >> 8) as u8, index: h as u8 }).collect(),
        sounds: (0..a.sounds.len() as u16).collect(),
        banners: small(a.banners.len()),
        backgrounds: small(a.backgrounds.len()),
        mugshots: small(a.mugshots.len()),
    }
}

/// `graphics` with a background on every number a round may draw, which
/// scrolls half a pixel left and a quarter up a frame, its columns colored
/// by their x: a picture that shows where its scenery is.
pub fn scrolling_background(graphics: &mut Graphics) {
    use nettai_assets::{Background, MapEntry, Tiles};
    let entry = |tile| MapEntry { tile, hflip: false, vflip: false, palette: 0 };
    let background = Background {
        tiles: Tiles { pixels: (0..2 * Tiles::TILE).map(|i| (1 + (i / 64) * 7 + i % 8) as u8).collect() },
        first_tile: 0x200,
        map: (0..32 * 32).map(|i| entry(0x200 + (i % 2) as u16)).collect(),
        map_width: 32,
        map_height: 32,
        palette: Some(std::array::from_fn(|i| i as u16 * 0x421)),
        scroll: (-8, -4),
        ..Background::default()
    };
    let mut bundle = (*graphics.bundles[0]).clone();
    bundle.backgrounds = vec![Some(background); 256];
    graphics.bundles[0] = Arc::new(bundle);
}

/// A battle's sound that makes a sound of every cue: a bank with a song
/// for each of `content`'s sounds, by its handle. Every fourth is a held
/// note on the music player (it goes on until another replaces it, as
/// music does); the others are short notes, each on one of sixteen
/// players at its own pitch.
pub fn audio(content: &Content) -> BattleAudio {
    BattleAudio::with_banks(vec![bank(content)], nettai_audio::Songs::numbers(content.assets.sounds.len()))
}

/// The bank of [`audio`]: a song for each of `content`'s sounds, by its
/// handle (its song number is its handle).
pub fn bank(content: &Content) -> Arc<m4a::SoundBank> {
    use m4a::bank::{Command, Envelope, MixerConfig, PlayerConfig, PlayerId, Song, Track, Voice, VoiceKind, Voicegroup, VoicegroupId};
    let square = Voice {
        kind: VoiceKind::Square1 { duty: 2, sweep: m4a::bank::NO_SWEEP, fixed: false },
        key: 60,
        pan: None,
        length: 0,
        envelope: Envelope { sustain: 15, ..Envelope::default() },
    };
    let count = content.assets.sounds.len();
    let songs = (0..count)
        .map(|n| {
            let key = Some(36 + (n % 48) as u8);
            let (player, commands) = if n % 4 == 0 {
                let held = vec![
                    Command::Tempo(75),
                    Command::Volume(100),
                    Command::Voice(0),
                    Command::Note { gate: 0, key, velocity: Some(100) },
                    Command::Wait(96),
                    Command::Goto(4),
                ];
                (nettai_audio::MUSIC_PLAYER, held)
            } else {
                let short =
                    vec![Command::Tempo(75), Command::Volume(100), Command::Voice(0), Command::Note { gate: 24, key, velocity: Some(100) }, Command::Wait(48), Command::Fine];
                (PlayerId((n % 16) as u8), short)
            };
            Some(Song { player, priority: 0, reverb: None, voicegroup: VoicegroupId(0), tracks: vec![Track { commands }] })
        })
        .collect();
    let players = (0..32).map(|p| PlayerConfig { max_tracks: if p == 31 { 8 } else { 2 }, uses_priority: false, track_order: p as u8 }).collect();
    let bank = m4a::SoundBank {
        mixer: MixerConfig { mix_rate: 10512, ds_channels: 4, master_volume: 15, reverb: 0, dac_resolution: 1 },
        players,
        songs,
        voicegroups: vec![Voicegroup { voices: vec![square] }],
        key_maps: vec![],
        samples: vec![],
        waves: vec![],
    };
    Arc::new(bank)
}
