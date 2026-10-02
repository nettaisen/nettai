//! Play songs or sound effects from a content pack's sound.
//!
//!     play_song <pack> <id>[,<id>...] [--every FRAMES] [--seconds S] [--wav OUT.wav]
//!
//! Starts each song-table entry (ids in hex or decimal; several are started
//! `--every` frames apart, 60 by default) on its music player and plays
//! for `--seconds` (5 by default), in real time or into a WAV file.

use nettai_audio::{AudioOut, BattleAudio, FPS, SAMPLE_RATE, SoundCue, SoundId, wav};
use std::time::{Duration, Instant};

fn usage() -> ! {
    eprintln!("usage: play_song <pack> <id>[,<id>...] [--every FRAMES] [--seconds S] [--wav OUT.wav]");
    std::process::exit(2);
}

fn parse_id(s: &str) -> u16 {
    let r = match s.strip_prefix("0x") {
        Some(h) => u16::from_str_radix(h, 16),
        None => s.parse(),
    };
    r.unwrap_or_else(|_| usage())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        usage();
    }
    let (bank, _) = nettai_content::pack::load_sound(std::path::Path::new(&args[0])).unwrap_or_else(|r| {
        eprintln!("{}: {r}", args[0]);
        std::process::exit(1);
    });
    let bank = std::sync::Arc::new(bank);
    let ids: Vec<u16> = args[1].split(',').map(parse_id).collect();
    let (mut every, mut seconds, mut out_path) = (60usize, 5.0f64, None);
    let mut i = 2;
    while i < args.len() {
        let v = args.get(i + 1).unwrap_or_else(|| usage());
        match args[i].as_str() {
            "--every" => every = v.parse().unwrap_or_else(|_| usage()),
            "--seconds" => seconds = v.parse().unwrap_or_else(|_| usage()),
            "--wav" => out_path = Some(v.clone()),
            _ => usage(),
        }
        i += 2;
    }
    for &id in &ids {
        match bank.song(m4a::SongId(id)) {
            Some(s) => {
                eprintln!("{id:#05x}: player {}, priority {}, {} tracks", s.player.0, s.priority, s.tracks.len())
            }
            None => eprintln!("{id:#05x}: no such song"),
        }
    }
    let frames = (seconds * FPS) as usize;
    let cues = |f: usize| -> Vec<SoundCue> {
        if f.is_multiple_of(every.max(1)) {
            ids.get(f / every.max(1)).map(|&id| SoundCue::Effect(SoundId(id))).into_iter().collect()
        } else {
            Vec::new()
        }
    };
    match out_path {
        Some(path) => {
            let mut audio = BattleAudio::new(bank.clone(), nettai_audio::Songs::numbers(bank.songs.len()));
            let mut samples = Vec::new();
            for f in 0..frames {
                audio.handle(&cues(f));
                audio.tick(&mut samples);
            }
            wav::write(&path, &samples, SAMPLE_RATE).unwrap_or_else(|e| panic!("writing {path}: {e}"));
            eprintln!("wrote {path}");
        }
        None => {
            let songs = nettai_audio::Songs::numbers(bank.songs.len());
            let mut out = AudioOut::new(bank, songs).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let mut next = Instant::now();
            for f in 0..frames {
                out.handle(&cues(f));
                out.tick();
                next += Duration::from_secs_f64(1.0 / FPS);
                if let Some(d) = next.checked_duration_since(Instant::now()) {
                    std::thread::sleep(d);
                }
            }
        }
    }
}
