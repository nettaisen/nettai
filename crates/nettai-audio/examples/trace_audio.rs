//! Replay a golden trace through the engine and play its sound.
//!
//!     trace_audio <trace.jsonl> <pack> [--round N] [--wav OUT.wav]
//!                 [--frames N] [--keep-going] [--tail SECONDS]
//!
//! Each round is replayed with the recorded inputs and its sound cues are
//! printed and played in real time (59.73 frames a second) on the default
//! output device. With `--wav`, the audio is rendered offline into a WAV
//! file instead (`--frames` caps how many frames). A round's replay stops
//! where the engine leaves the recording (or, with `--keep-going`, where it
//! panics on something it doesn't implement yet).
//!
//! The content pack (the assets, among them the sound) comes from
//! `bn6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <pack>`; the battle content the engine runs
//! on is this repository's content/bn6 (or `$BN6_CONTENT`).

use nettai_audio::{AudioOut, BattleAudio, FPS, SAMPLE_RATE, SoundCue, wav};
use bn6_compat::trace;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Where the audio goes: rendered into memory for a WAV file, or played on
/// the device paced to the GBA's frame rate.
enum Sink {
    Offline { audio: BattleAudio, samples: Vec<[f32; 2]> },
    Live { out: AudioOut, next: Instant },
}

impl Sink {
    fn frame(&mut self, cues: &[SoundCue]) {
        match self {
            Sink::Offline { audio, samples } => {
                audio.handle(cues);
                audio.tick(samples);
            }
            Sink::Live { out, next } => {
                out.handle(cues);
                out.tick();
                *next += Duration::from_secs_f64(1.0 / FPS);
                let now = Instant::now();
                if *next > now {
                    std::thread::sleep(*next - now);
                } else if now - *next > Duration::from_millis(200) {
                    // Fell far behind (a stall): don't rush to catch up.
                    *next = now;
                }
            }
        }
    }
}

struct Options {
    trace: String,
    pack: String,
    round: Option<usize>,
    wav: Option<String>,
    frames: Option<usize>,
    keep_going: bool,
    tail: f64,
}

fn usage() -> ! {
    eprintln!(
        "usage: trace_audio <trace.jsonl> <pack> [--round N] [--wav OUT.wav] [--frames N] [--keep-going] [--tail SECONDS]"
    );
    std::process::exit(2);
}

fn options() -> Options {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut o = Options {
        trace: String::new(),
        pack: String::new(),
        round: None,
        wav: None,
        frames: None,
        keep_going: false,
        tail: 1.0,
    };
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let value = |i: usize| args.get(i + 1).cloned().unwrap_or_else(|| usage());
        match args[i].as_str() {
            "--round" => o.round = Some(value(i).parse().unwrap_or_else(|_| usage())),
            "--wav" => o.wav = Some(value(i)),
            "--frames" => o.frames = Some(value(i).parse().unwrap_or_else(|_| usage())),
            "--tail" => o.tail = value(i).parse().unwrap_or_else(|_| usage()),
            "--keep-going" => {
                o.keep_going = true;
                i += 1;
                continue;
            }
            a if a.starts_with("--") => usage(),
            a => {
                positional.push(a.to_string());
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    let [trace, pack] = <[String; 2]>::try_from(positional).unwrap_or_else(|_| usage());
    o.trace = trace;
    o.pack = pack;
    o
}

fn describe(c: &SoundCue) -> String {
    match c {
        SoundCue::Effect(id) => format!("effect {:#05x}", id.0),
        SoundCue::Music(id) => format!("music {:#04x}", id.0),
        SoundCue::StopMusic => "stop music".into(),
        SoundCue::Pinch(on) => format!("pinch {}", if *on { "on" } else { "off" }),
        SoundCue::RestoreVolume => "restore volume".into(),
        SoundCue::ScreenVolume { music, screen } => format!("volume: music {music:#x}, custom screen {screen:#x}"),
    }
}

fn main() {
    let o = options();
    let fail = |r: nettai_content::report::Report| -> ! {
        eprintln!("{}: {r}
(write a content pack with `bn6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <pack>`)", o.pack);
        std::process::exit(1);
    };
    let (bank, _) = nettai_content::pack::load_sound(Path::new(&o.pack)).unwrap_or_else(|r| fail(r));
    let bank = Arc::new(bank);
    let (content, _) = nettai_content::pack::load_battle(&nettai_content::root::bn6(), Path::new(&o.pack)).unwrap_or_else(|r| fail(r));
    let content = Arc::new(content);
    let rounds = trace::rounds(&o.trace).unwrap_or_else(|e| {
        eprintln!("{}: {e}", o.trace);
        std::process::exit(1);
    });
    let mut sink = match &o.wav {
        Some(_) => Sink::Offline { audio: BattleAudio::new(bank), samples: Vec::new() },
        None => {
            let out = AudioOut::new(bank).unwrap_or_else(|e| {
                eprintln!("{e} (use --wav to render to a file)");
                std::process::exit(1);
            });
            Sink::Live { out, next: Instant::now() }
        }
    };
    // The engine's panics are reported below.
    std::panic::set_hook(Box::new(|_| {}));
    let mut budget = o.frames.unwrap_or(usize::MAX);
    for (n, round) in rounds.iter().enumerate() {
        if o.round.is_some_and(|r| r != n + 1) || budget == 0 {
            continue;
        }
        let frames: Vec<&trace::Frame> = round.battle_frames().collect();
        let compat = bn6_compat::Compat::bn6();
        let ids = bn6_compat::codec::Ids::new(&content, compat);
        let mut b = round.start(content.clone(), compat);
        eprintln!("round {}: {} frames from frame {}", n + 1, frames.len(), round.setup.frame);
        for i in 0..frames.len() {
            if budget == 0 {
                break;
            }
            budget -= 1;
            let frame = frames[i].frame;
            let (input, events) = round.tick_inputs(i, &frames, &ids);
            let ticked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(&input, events)));
            if let Err(e) = ticked {
                let msg =
                    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
                eprintln!("  frame {frame}: the engine stopped: {}", msg.unwrap_or_default());
                break;
            }
            for c in b.sound_cues() {
                eprintln!("  frame {frame}: {}", describe(c));
            }
            sink.frame(b.sound_cues());
            if !o.keep_going {
                let diffs = trace::compare(&b, frames[i], bn6_compat::Compat::bn6());
                if let Some(d) = diffs.first() {
                    eprintln!("  frame {frame}: the engine leaves the recording ({})", d.lines().next().unwrap_or(""));
                    break;
                }
            }
        }
    }
    for _ in 0..(o.tail * FPS) as usize {
        sink.frame(&[]);
    }
    if let (Some(path), Sink::Offline { samples, .. }) = (&o.wav, &sink) {
        wav::write(path, samples, SAMPLE_RATE).unwrap_or_else(|e| panic!("writing {path}: {e}"));
        eprintln!("wrote {path} ({:.1} s)", samples.len() as f64 / SAMPLE_RATE as f64);
    }
}
