//! bn6-frontend: watch a golden trace replayed through the engine, or play.
//! See docs/frontend.md.

use bn6_assets::Bundle;
use bn6_frontend::driver::{LivePlayer, TracePlayer, live_setup};
use bn6_frontend::{Renderer, Session, TickHook, app, headless, session};
use std::path::PathBuf;

struct Args {
    graphics: Option<PathBuf>,
    sound: Option<PathBuf>,
    trace: Option<PathBuf>,
    round: usize,
    play: bool,
    seed: Option<u32>,
    scale: usize,
    paused: bool,
    headless: Option<String>,
    out: PathBuf,
    png_scale: usize,
    quit_after: Option<u64>,
}

/// Where `bn6-extract graphics` puts the bundle by default.
const DEFAULT_GRAPHICS: &str = "data/graphics";

const USAGE: &str = "\
usage: bn6-frontend [OPTIONS] TRACE.jsonl     watch a trace's rounds
       bn6-frontend [OPTIONS] --play          play live (you are the left navi)
       bn6-frontend [OPTIONS] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]

  --graphics PATH  a content pack (from `bn6-extract content <rom> <dir>`), or
                   the graphics bundle or its directory, from
                   `bn6-extract graphics <rom> <dir>` (default: $BN6_GRAPHICS,
                   else data/graphics)
  --sound BANK     play sound with a content pack's sound, or the bank from
                   `bn6-extract assets <rom> BANK`
  --round N        the trace round to start with (default 1; later rounds follow)
  --seed N         the live battle's RNG seed
  --scale N        window scale (default 4)
  --paused         start paused
  --headless F     render frames F (e.g. 150,300,600 or 100-120; trace frame
                   numbers, or ticks in live play) to frame_NNNNN.png files
                   in --out (default .), no window
  --quit-after N   close the window after N ticks";

fn parse() -> Result<Args, String> {
    let mut a = Args {
        graphics: None,
        sound: None,
        trace: None,
        round: 1,
        play: false,
        seed: None,
        scale: 4,
        paused: false,
        headless: None,
        out: PathBuf::from("."),
        png_scale: 1,
        quit_after: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        let number = |v: String, name: &str| v.parse::<u64>().map_err(|_| format!("bad {name} {v:?}"));
        match arg.as_str() {
            "--graphics" => a.graphics = Some(value("--graphics")?.into()),
            "--sound" => a.sound = Some(value("--sound")?.into()),
            "--round" => a.round = number(value("--round")?, "--round")? as usize,
            "--play" => a.play = true,
            "--seed" => a.seed = Some(number(value("--seed")?, "--seed")? as u32),
            "--scale" => a.scale = number(value("--scale")?, "--scale")? as usize,
            "--paused" => a.paused = true,
            "--headless" => a.headless = Some(value("--headless")?),
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

/// A content pack's derived data lives in its `.cache` folder.
fn pack_cache(pack: &std::path::Path) -> PathBuf {
    pack.join(".cache")
}

fn is_pack(path: &std::path::Path) -> bool {
    path.join(bn6_content::pack::MANIFEST).is_file()
}

/// Show what building a pack's data found (warnings and errors).
fn show(report: &bn6_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != bn6_content::report::Level::Note) {
        eprintln!("{i}");
    }
}

fn load_graphics(path: &std::path::Path) -> Bundle {
    if is_pack(path) {
        let (b, r) = bn6_content::pack::load_graphics(path, &pack_cache(path)).unwrap_or_else(|r| {
            show(&r);
            fail(format!("can't build the graphics of the content pack {}", path.display()))
        });
        show(&r);
        return b;
    }
    Bundle::load(path).unwrap_or_else(|e| {
        fail(format!(
            "can't load the graphics from {}: {e}\n(create them with `cargo run -p bn6-extract -- content <rom> <dir>`)",
            path.display()
        ))
    })
}

/// Sound: hand each tick's cues to the audio output.
fn audio_hook(bank: &std::path::Path) -> Box<dyn TickHook> {
    let bank = if is_pack(bank) {
        let (b, r) = bn6_content::pack::load_sound(bank, &pack_cache(bank)).unwrap_or_else(|r| {
            show(&r);
            fail(format!("can't build the sound of the content pack {}", bank.display()))
        });
        show(&r);
        std::sync::Arc::new(b)
    } else {
        bn6_audio::load_bank(bank).unwrap_or_else(|e| fail(format!("can't load the sound bank {}: {e}", bank.display())))
    };
    let mut out = bn6_audio::AudioOut::new(bank).unwrap_or_else(|e| fail(format!("no audio output: {e}")));
    Box::new(move |b: &bn6_battle::Battle| {
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
    let graphics = args
        .graphics
        .clone()
        .or_else(|| std::env::var_os("BN6_GRAPHICS").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GRAPHICS));
    let assets = load_graphics(&graphics);
    session::quiet_engine_panics();
    let mut renderer = Renderer::new(&assets);

    let mut sessions: Vec<Session> = Vec::new();
    if args.play {
        let seed = args.seed.unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
        });
        sessions.push(Session::new(Box::new(LivePlayer::new(live_setup(seed)))));
    } else if let Some(path) = &args.trace {
        let rounds = TracePlayer::load(path).unwrap_or_else(|e| fail(format!("can't read {}: {e}", path.display())));
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

    if let Some(list) = &args.headless {
        let wanted = headless::parse_frames(list).unwrap_or_else(|e| fail(e));
        let mut log = |s: &str| eprintln!("{s}");
        match headless::render_frames(&mut renderer, sessions, &wanted, &args.out, args.png_scale, &mut log) {
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
    if let Some(bank) = &args.sound {
        hooks.push(audio_hook(bank));
    }
    eprintln!("{}", app::HELP);
    let opts = app::Options { scale: args.scale, start_paused: args.paused, quit_after: args.quit_after };
    if let Err(e) = app::run(&mut renderer, sessions, &mut hooks, &opts) {
        fail(format!("window: {e}"));
    }
}
