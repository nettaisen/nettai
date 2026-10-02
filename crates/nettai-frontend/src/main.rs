//! nettai-frontend: watch a golden trace replayed through the engine, or play.
//! See docs/frontend.md.

use nettai_frontend::driver::{LivePlayer, TracePlayer, bn6_live_setup};
use nettai_frontend::{Renderer, Session, TickHook, app, headless, session};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

struct Args {
    pack: Option<PathBuf>,
    content: Option<PathBuf>,
    mute: bool,
    trace: Option<PathBuf>,
    round: usize,
    play: bool,
    seed: Option<u32>,
    scale: usize,
    paused: bool,
    headless: Option<String>,
    audit: bool,
    objects: bool,
    out: PathBuf,
    png_scale: usize,
    quit_after: Option<u64>,
}

/// Where `bn6-extract content <rom> <dir>` puts the BN6 pack by default.
const DEFAULT_PACK: &str = "data/content/bn6";

const USAGE: &str = "\
usage: nettai-frontend [OPTIONS] TRACE.jsonl     watch a trace's rounds
       nettai-frontend [OPTIONS] --play          play live (you are the left navi)
       nettai-frontend [OPTIONS] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]
       nettai-frontend [OPTIONS] TRACE.jsonl --audit

  --pack DIR       the content pack to play (graphics and sound), from
                   `bn6-extract content <rom> <dir>` (default: $BN6_PACK, else
                   data/content/bn6)
  --content DIR    the battle content: the definitions that name the pack's
                   assets (default: $BN6_CONTENT, else this repository's
                   content/bn6)
  --mute           no sound (headless rendering never plays any)
  --round N        the trace round to start with (default 1; later rounds follow)
  --seed N         the live battle's RNG seed
  --scale N        window scale (default 4)
  --paused         start paused
  --headless F     render frames F (e.g. 150,300,600 or 100-120; trace frame
                   numbers, or ticks in live play) to frame_NNNNN.png files
                   in --out (default .), no window
  --objects        with --headless: list every rendered frame's objects (kind,
                   place, sprite, animation, look)
  --audit          draw every frame and play every sound cue into nothing, no
                   window, and list what they named that the pack doesn't
                   have (a sprite, an animation, a palette, a chip's icon or
                   name glyph, a banner, a song); exits 1 if there was any
  --quit-after N   close the window after N ticks";

fn parse() -> Result<Args, String> {
    let mut a = Args {
        pack: None,
        content: None,
        mute: false,
        trace: None,
        round: 1,
        play: false,
        seed: None,
        scale: 4,
        paused: false,
        headless: None,
        audit: false,
        objects: false,
        out: PathBuf::from("."),
        png_scale: 1,
        quit_after: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        let number = |v: String, name: &str| v.parse::<u64>().map_err(|_| format!("bad {name} {v:?}"));
        match arg.as_str() {
            "--pack" => a.pack = Some(value("--pack")?.into()),
            "--content" => a.content = Some(value("--content")?.into()),
            "--mute" => a.mute = true,
            "--round" => a.round = number(value("--round")?, "--round")? as usize,
            "--play" => a.play = true,
            "--seed" => a.seed = Some(number(value("--seed")?, "--seed")? as u32),
            "--scale" => a.scale = number(value("--scale")?, "--scale")? as usize,
            "--paused" => a.paused = true,
            "--headless" => a.headless = Some(value("--headless")?),
            "--audit" => a.audit = true,
            "--objects" => a.objects = true,
            "--out" => a.out = value("--out")?.into(),
            "--png-scale" => a.png_scale = number(value("--png-scale")?, "--png-scale")? as usize,
            "--quit-after" => a.quit_after = Some(number(value("--quit-after")?, "--quit-after")?),
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => a.trace = Some(s.into()),
        }
    }
    if a.trace.is_none() && !a.play {
        return Err("give a trace file or --play".into());
    }
    Ok(a)
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

/// Show what loading a pack found (warnings and errors).
fn show(report: &nettai_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != nettai_content::report::Level::Note) {
        eprintln!("{i}");
    }
}

/// Load one part of the pack, or stop with what is wrong with it.
fn load<T>(pack: &Path, what: &str, f: impl Fn(&Path) -> Result<(T, nettai_content::report::Report), nettai_content::report::Report>) -> T {
    let t = Instant::now();
    match f(pack) {
        Ok((v, r)) => {
            show(&r);
            if std::env::var_os("NETTAI_LOAD_TIMES").is_some() {
                eprintln!("loaded the {what} in {:.1?}", t.elapsed());
            }
            v
        }
        Err(r) => {
            show(&r);
            fail(format!(
                "can't load the {what} of the content pack {}\n(write it with `cargo run -p bn6-extract -- content <rom> {}`)",
                pack.display(),
                pack.display()
            ))
        }
    }
}

/// Sound: hand each tick's cues to the audio output.
fn audio_hook(bank: m4a::SoundBank) -> Box<dyn TickHook> {
    let mut out = nettai_audio::AudioOut::new(Arc::new(bank)).unwrap_or_else(|e| fail(format!("no audio output: {e}")));
    Box::new(move |b: &nettai_battle::Battle| {
        out.handle(b.sound_cues());
        out.tick();
    })
}

fn main() {
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprintln!("{USAGE}\n\n{}", app::HELP);
            std::process::exit(2);
        }
    };
    let pack = args
        .pack
        .clone()
        .or_else(|| std::env::var_os("BN6_PACK").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PACK));
    let root = args.content.clone().unwrap_or_else(nettai_content::root::bn6);
    let content = Arc::new(load(&pack, "battle content", |pack| nettai_content::pack::load_battle(&root, pack)));
    let assets = load(&pack, "graphics", nettai_content::pack::load_graphics);
    session::quiet_engine_panics();
    let mut renderer = Renderer::new(&assets);

    let mut sessions: Vec<Session> = Vec::new();
    if args.play {
        let seed = args.seed.unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
        });
        sessions.push(Session::new(Box::new(LivePlayer::new(bn6_live_setup(&content, seed), content.clone()))));
    } else if let Some(path) = &args.trace {
        let rounds =
            TracePlayer::load(path, &content).unwrap_or_else(|e| fail(format!("can't read {}: {e}", path.display())));
        for r in rounds.into_iter().skip(args.round.saturating_sub(1)) {
            if let Some((a, b)) = r.frame_range() {
                eprintln!("round {}: frames {a}..={b}", r.round_number);
            }
            sessions.push(Session::new(Box::new(r)));
        }
    }
    if sessions.is_empty() {
        fail("nothing to play");
    }

    if args.audit {
        let sound = (!args.mute).then(|| Arc::new(load(&pack, "sound", nettai_content::pack::load_sound)));
        let found = headless::audit(&mut renderer, sessions, sound);
        for s in &found.stopped {
            eprintln!("{s}");
        }
        for line in found.problems.lines() {
            println!("{line}");
        }
        eprintln!("audit: {} frames, {} sound cues, {} problems", found.frames, found.cues, found.problems.len());
        if !found.problems.is_empty() {
            std::process::exit(1);
        }
        return;
    }
    if let Some(list) = &args.headless {
        let wanted = headless::parse_frames(list).unwrap_or_else(|e| fail(e));
        let mut log = |s: &str| eprintln!("{s}");
        let rendered =
            headless::render_frames_with(&mut renderer, sessions, &wanted, &args.out, args.png_scale, args.objects, &mut log);
        match rendered {
            Ok(written) => {
                eprintln!("wrote {} frames to {}", written.len(), args.out.display());
                if written.len() < wanted.len() {
                    std::process::exit(1);
                }
            }
            Err(e) => fail(e),
        }
        return;
    }

    let mut hooks: Vec<Box<dyn TickHook>> = Vec::new();
    if !args.mute {
        hooks.push(audio_hook(load(&pack, "sound", nettai_content::pack::load_sound)));
    }
    eprintln!("{}", app::HELP);
    let opts = app::Options { scale: args.scale, start_paused: args.paused, quit_after: args.quit_after };
    if let Err(e) = app::run(&mut renderer, sessions, &mut hooks, &opts) {
        fail(format!("window: {e}"));
    }
}
