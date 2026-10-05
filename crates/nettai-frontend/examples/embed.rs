//! Embedding nettai-frontend: a host with no window and no audio device.
//! It loads a game, plays a seeded random match with scripted buttons a
//! tick at a time, and takes each tick's picture and sound from the player,
//! as an app would for its own screen and speaker.
//!
//! It checks what it took: every frame against the PNG the desktop program
//! wrote for the same match and buttons, byte for byte, and the samples
//! against a `BattleAudio` it feeds the battle's cues itself (the program's
//! headless output has no sound to compare with).
//!
//! ```text
//! nettai-demo --play --game exe6 --seed 7 --mute --headless 100,400,700,900 --out DIR \
//!     --keys "$(cargo run -p nettai-frontend --example embed -- --keys)"
//! cargo run -p nettai-frontend --example embed -- --game exe6 --seed 7 --frames DIR
//! ```
//!
//! The packs are found as the program finds them (`$NETTAI_PACKS`).

use nettai_audio::BattleAudio;
use nettai_battle::input::keys;
use nettai_frontend::Player;
use nettai_frontend::driver::LivePlayer;
use nettai_frontend::game::{self, Options};
use std::path::{Path, PathBuf};

/// The buttons held, by tick: the first custom screen's pick and OK, then a
/// chip, the buster and a step. (`--keys` prints it as the program's
/// `--keys` takes it.)
const SCRIPT: &[(u32, u32, &str, u16)] = &[
    (300, 340, "a", keys::A),
    (420, 420, "start", keys::START),
    (430, 460, "a", keys::A),
    (700, 700, "a", keys::A),
    (760, 800, "b", keys::B),
    (820, 823, "right", keys::RIGHT),
];

fn held(tick: u32) -> u16 {
    SCRIPT.iter().filter(|(from, to, ..)| (*from..=*to).contains(&tick)).fold(0, |all, (.., button)| all | button)
}

/// The frames a directory of the program's headless output holds
/// (`frame_NNNNN.png`), by number.
fn frames_in(dir: &Path) -> Result<Vec<(u32, PathBuf)>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("can't read {}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let number = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_prefix("frame_")?.strip_suffix(".png")?.parse().ok());
        if let Some(n) = number {
            out.push((n, path));
        }
    }
    out.sort();
    Ok(out)
}

fn run() -> Result<String, String> {
    let (mut name, mut seed, mut frames) = (None, None, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--keys" => {
                let parts: Vec<String> =
                    SCRIPT.iter().map(|(from, to, button, _)| if from == to { format!("{from}:{button}") } else { format!("{from}-{to}:{button}") }).collect();
                return Ok(parts.join(","));
            }
            "--game" => name = Some(value()?),
            "--seed" => seed = Some(value()?.parse::<u32>().map_err(|e| format!("--seed: {e}"))?),
            "--frames" => frames = Some(PathBuf::from(value()?)),
            other => return Err(format!("no option {other:?} (--game GAME --seed N --frames DIR, or --keys)")),
        }
    }
    let name = name.ok_or("say which game: --game exe6 or --game exe5")?;
    let seed = seed.ok_or("say the match's seed: --seed N")?;
    let frames = frames_in(&frames.ok_or("say where the program's headless frames are: --frames DIR")?)?;
    let last = frames.last().ok_or("no frame_NNNNN.png there")?.0;

    // Loading: one call. What the loaders had to say is the host's to show.
    let game = game::load(&name, &Options::default()).map_err(|e| format!("{}\n{e}", e.report))?;
    let content = &game.game.content;
    // The match the program picks for the seed, and a live driver of its set.
    let m = nettai_match::pick::live(content, &name, seed, None)?;
    let driver = LivePlayer::new(nettai_match::Set::of(content, &m, seed));
    let mut player = Player::new(&game, Box::new(driver));

    // A second rendition of the sound, from the battle's cues, to compare.
    let sound = game.sound.as_ref().ok_or("the game loaded without sound")?;
    let mut reference = BattleAudio::with_banks(sound.banks.clone(), sound.songs.clone());
    let (mut samples, mut expected) = (Vec::new(), Vec::new());
    let (mut total, mut loudest) = (0usize, 0f32);

    let scratch = std::env::temp_dir().join(format!("nettai-embed-{}.png", std::process::id()));
    let mut wanted = frames.iter().peekable();
    let mut same = 0;
    for tick in 1..=last {
        // The host's part of a frame: the buttons in, a tick, the picture and the sound out.
        if !player.tick(held(tick)) {
            return Err(format!("the battle stopped at tick {tick}: {}", player.stopped().unwrap_or("?")));
        }
        samples.clear();
        player.take_samples(&mut samples);
        expected.clear();
        reference.handle(player.battle().sound_cues());
        reference.tick(&mut expected);
        if samples != expected {
            return Err(format!("tick {tick}: the player's {} samples aren't the cues' own {}", samples.len(), expected.len()));
        }
        total += samples.len();
        loudest = samples.iter().flatten().fold(loudest, |m, s| m.max(s.abs()));
        if let Some((_, theirs)) = wanted.next_if(|(n, _)| *n == tick) {
            let frame = player.frame();
            nettai_render::present::write_png(&scratch, &frame, 1, player.text()).map_err(|e| e.to_string())?;
            let (ours, theirs_bytes) = (std::fs::read(&scratch), std::fs::read(theirs));
            let _ = std::fs::remove_file(&scratch);
            if ours.map_err(|e| e.to_string())? != theirs_bytes.map_err(|e| e.to_string())? {
                return Err(format!("frame {tick} isn't the program's ({})", theirs.display()));
            }
            same += 1;
        }
    }
    // The same picture into a host's own pixels, at a size of its choosing.
    let (w, h) = (640, 480);
    let mut pixels = vec![0u32; w * h];
    player.present(&mut pixels, w, h);
    if loudest == 0.0 {
        return Err("the match made no sound".into());
    }
    Ok(format!(
        "{name} seed {seed}: {last} ticks; {same} frames identical to the program's; {total} samples identical to the cues' own (loudest {loudest:.2}); \
         presented into {w}x{h}; {}",
        player.position()
    ))
}

fn main() {
    match run() {
        Ok(said) => println!("{said}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
