//! bn6-frontend: watch a golden trace replayed through the engine, or play.
//!
//! Usage:
//!   bn6-frontend [--assets PATH] TRACE.jsonl [--round N] [--scale N] [--paused]
//!   bn6-frontend [--assets PATH] --play [--seed N] [--scale N]
//!   bn6-frontend [--assets PATH] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]
//!
//! The asset bundle comes from `bn6-extract assets <rom> <dir>`; PATH is
//! that directory or the bundle file (default: $BN6_ASSETS, else ./assets).

use bn6_assets::Bundle;
use bn6_frontend::driver::{LivePlayer, TracePlayer, live_setup};
use bn6_frontend::{Renderer, Session, app, headless, session};
use std::path::PathBuf;

struct Args {
    assets: Option<PathBuf>,
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

const USAGE: &str = "\
usage: bn6-frontend [--assets PATH] TRACE.jsonl [--round N] [--scale N] [--paused]
       bn6-frontend [--assets PATH] --play [--seed N] [--scale N]
       bn6-frontend [--assets PATH] TRACE.jsonl --headless FRAMES [--out DIR] [--png-scale N]

  --assets PATH   the asset bundle or its directory (from `bn6-extract assets
                  ROM DIR`); default $BN6_ASSETS, else ./assets
  --round N       the trace round to start with (default 1; later rounds follow)
  --play          play live: you are the left navi
  --headless F    render trace frames F (e.g. 150,300,600 or 100-120) to PNG
                  files frame_NNNNN.png in --out (default .), no window
  --quit-after N  close the window after N ticks";

fn parse() -> Result<Args, String> {
    let mut a = Args {
        assets: None,
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
        match arg.as_str() {
            "--assets" => a.assets = Some(value("--assets")?.into()),
            "--round" => a.round = value("--round")?.parse().map_err(|_| "bad --round")?,
            "--play" => a.play = true,
            "--seed" => a.seed = Some(value("--seed")?.parse().map_err(|_| "bad --seed")?),
            "--scale" => a.scale = value("--scale")?.parse().map_err(|_| "bad --scale")?,
            "--paused" => a.paused = true,
            "--headless" => a.headless = Some(value("--headless")?),
            "--out" => a.out = value("--out")?.into(),
            "--png-scale" => a.png_scale = value("--png-scale")?.parse().map_err(|_| "bad --png-scale")?,
            "--quit-after" => a.quit_after = Some(value("--quit-after")?.parse().map_err(|_| "bad --quit-after")?),
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
    let assets_path = args
        .assets
        .clone()
        .or_else(|| std::env::var_os("BN6_ASSETS").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("assets"));
    let assets = match Bundle::load(&assets_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "can't load the asset bundle from {}: {e}\n(create it with `cargo run -p bn6-extract -- assets <rom> <dir>`)",
                assets_path.display()
            );
            std::process::exit(1);
        }
    };
    session::quiet_engine_panics();
    let mut renderer = Renderer::new(&assets);

    let mut sessions: Vec<Session> = Vec::new();
    if args.play {
        let seed = args.seed.unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
        });
        sessions.push(Session::new(Box::new(LivePlayer::new(live_setup(seed)))));
    } else if let Some(path) = &args.trace {
        let rounds = match TracePlayer::load(path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("can't read {}: {e}", path.display());
                std::process::exit(1);
            }
        };
        for r in rounds.into_iter().skip(args.round.saturating_sub(1)) {
            if let Some((a, b)) = r.frame_range() {
                eprintln!("round {}: frames {a}..={b}", r.round_number);
            }
            sessions.push(Session::new(Box::new(r)));
        }
    }
    if sessions.is_empty() {
        eprintln!("nothing to play");
        std::process::exit(1);
    }

    if let Some(list) = &args.headless {
        let wanted = match headless::parse_frames(list) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        };
        let mut log = |s: &str| eprintln!("{s}");
        match headless::render_frames(&mut renderer, sessions, &wanted, &args.out, args.png_scale, &mut log) {
            Ok(written) => {
                eprintln!("wrote {} frames to {}", written.len(), args.out.display());
                if written.len() < wanted.len() {
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }

    eprintln!("{}", app::HELP);
    let opts = app::Options { scale: args.scale, start_paused: args.paused, quit_after: args.quit_after };
    if let Err(e) = app::run(&mut renderer, sessions, &opts) {
        eprintln!("window: {e}");
        std::process::exit(1);
    }
}
