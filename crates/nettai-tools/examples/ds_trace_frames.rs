//! The engine's frames of a DS recording (EXE5 DS's party battle, a dslab
//! recording: `"game": "exe5ds"`), drawn from EXE5's assets (extracted from
//! its ROMs: `$NETTAI_ROMS`) as exe5-compat
//! replays it, for a frame comparison with the DS's captures (the
//! verification workspace's dslab `--frames`). The frontend's trace player
//! doesn't play DS recordings; this replays them as the verification's
//! exe5_replay does, up to the first difference.
//!
//!     cargo run -p nettai-tools --example ds_trace_frames -- TRACE OUT [FIRST LAST]
//!
//! Each battle frame from FIRST to LAST (the recording's frame numbers; all
//! by default) is drawn (original graphics, 1x) as `OUT/nettai_NNNNN.png`,
//! NNNNN its frame number, as the DS's captures are named.

use exe5_compat::trace;
use nettai_frontend::game::{Options, load};
use nettai_render::GraphicsMode;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: ds_trace_frames TRACE OUT [FIRST LAST]";
    if args.len() != 2 && args.len() != 4 {
        panic!("{usage}");
    }
    let (path, out) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    let (first, last) = if args.len() == 4 {
        (args[2].parse::<u32>().expect(usage), args[3].parse::<u32>().expect(usage))
    } else {
        (0, u32::MAX)
    };
    std::fs::create_dir_all(&out).unwrap();
    let options = Options { mode: GraphicsMode::Original, sound: false, ..Options::default() };
    let loaded = load("exe5", &options).unwrap_or_else(|e| panic!("exe5: {e:?}"));
    let content = loaded.game.content.clone();
    let mut renderer = loaded.renderer();
    let compat = exe5_compat::Compat::exe5();
    let rounds = trace::rounds(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    for (i, round) in rounds.iter().enumerate() {
        let replay = trace::run_round_seeing(round, &content, compat, &mut |frame, b| {
            if (first..=last).contains(&frame) {
                let picture = renderer.render(b);
                nettai_render::present::write_png(&out.join(format!("nettai_{frame:05}.png")), &picture, 1, None).unwrap();
            }
        });
        println!("round {}: {} of {} frames matched; {:?}", i + 1, replay.matched, replay.frames, replay.stopped);
    }
}
