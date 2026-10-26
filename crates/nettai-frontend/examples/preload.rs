//! Loading and preloading, held to what they say, on a real pack: a game is
//! loaded a part at a time (`game::Loading`), a match of it made ready a
//! step at a time (`preload::Preload`), and then **the pack and the content
//! are deleted from disk** before the match is played. A match that read
//! or decoded a file on the way would stop or draw nothing; this one plays
//! to its last tick with every picture presented and every sample taken,
//! each the same as a player's made without the preload, and its player's
//! count of what it made late (`Player::late`) is no runtime loaded and
//! nothing read.
//!
//! It prints what each part cost, so the figures in docs/frontend.md §9 can
//! be measured again: each part of the game's load, the thread's runtime,
//! each step of the preload, the worst tick and picture of the match, and
//! what the same match costs a thread that preloaded nothing.
//!
//! ```text
//! cargo run --release -p nettai-frontend --example preload -- --game exe6
//! ```
//!
//! The packs are found as the tool finds them (`$NETTAI_PACKS`); the game's
//! pack and the content directory are copied to the system's temporary
//! directory, and the copies are what is loaded and deleted.

use nettai_battle::battle::mode;
use nettai_frontend::Player;
use nettai_frontend::driver::{InFight, LivePlayer, Opponent, bot_buttons};
use nettai_frontend::game::{Found, Loading, Options};
use nettai_frontend::preload::{self, Part, Preload};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Copy a directory and everything under it.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<u64> {
    std::fs::create_dir_all(to)?;
    let mut files = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            files += copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
            files += 1;
        }
    }
    Ok(files)
}

/// The driver of the match: the set against an opponent that attacks at
/// random, the same each time.
fn driver(set: &nettai_match::Set) -> Box<LivePlayer> {
    Box::new(LivePlayer::against(set.clone(), Opponent { in_fight: InFight::AttacksAtRandom, ..Opponent::default() }, 3))
}

/// Play `ticks` ticks of `player`'s set, each picture presented at `size`
/// and its sound taken: the local player presses through the custom
/// screens and mashes in the fight. `plain`, a player of the same set made
/// without the preload, is given the same buttons, and each tick's picture
/// (its pixels and its text) and sound must be the same as its. The longest
/// tick and picture of `player`'s.
fn play(player: &mut Player, plain: &mut Player, ticks: u32, (w, h): (usize, usize)) -> Result<(Duration, Duration), String> {
    let mut pixels = vec![0u32; w * h];
    let (mut samples, mut plain_samples) = (Vec::new(), Vec::new());
    let mut masher = nettai_netplay::standin::Masher::new(5);
    masher.buster = true;
    let (mut longest_tick, mut longest_picture) = (Duration::ZERO, Duration::ZERO);
    for tick in 0..ticks {
        let b = player.battle();
        let keys = if b.round.mode == mode::CUSTOM { bot_buttons(b, 0, tick) } else { masher.buttons() };
        let t = Instant::now();
        if !player.tick(keys) {
            // (A set played out is its end, not a stop.)
            if player.finished() {
                break;
            }
            return Err(format!("the match stopped at tick {tick}: {}", player.stopped().unwrap_or("?")));
        }
        longest_tick = longest_tick.max(t.elapsed());
        let t = Instant::now();
        player.present(&mut pixels, w, h);
        longest_picture = longest_picture.max(t.elapsed());
        samples.clear();
        player.take_samples(&mut samples);
        // The same tick of the player made without the preload.
        plain.tick(keys);
        let (ours, theirs) = (player.frame(), plain.frame());
        if ours.pixels != theirs.pixels || ours.text != theirs.text {
            return Err(format!("tick {tick}: the preloaded player's picture isn't the one a player made without the preload draws"));
        }
        plain_samples.clear();
        plain.take_samples(&mut plain_samples);
        if samples != plain_samples {
            return Err(format!("tick {tick}: the preloaded player's sound isn't the one a player made without the preload makes"));
        }
    }
    Ok((longest_tick, longest_picture))
}

fn run() -> Result<(), String> {
    let (mut name, mut seed, mut ticks, mut size) = (None, 7u32, 3000u32, (720usize, 480usize));
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--game" => name = Some(value()?),
            "--seed" => seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--ticks" => ticks = value()?.parse().map_err(|e| format!("--ticks: {e}"))?,
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size WIDTHxHEIGHT")?;
                size = (w.parse().map_err(|e| format!("--size: {e}"))?, h.parse().map_err(|e| format!("--size: {e}"))?);
            }
            other => return Err(format!("no option {other:?} (--game GAME [--seed N] [--ticks N] [--size WIDTHxHEIGHT])")),
        }
    }
    let name = name.ok_or("say which game: --game exe6, exe5 or exe4")?;

    // Copies of the game's pack and of the content, to load and then delete.
    let found = Found::find(&nettai_content::pack::packs_dir(), &[]).map_err(|e| format!("{}\n{e}", e.report))?;
    let pack = found.packs.iter().find(|p| p.game == name).ok_or(format!("no pack of {name} in {}", nettai_content::pack::packs_dir().display()))?;
    let scratch = std::env::temp_dir().join(format!("nettai-preload-{}", std::process::id()));
    let copies: (PathBuf, PathBuf) = (scratch.join("pack"), scratch.join("content"));
    let copied = copy_dir(&pack.dir, &copies.0).and_then(|a| Ok(a + copy_dir(&nettai_content::index::content(), &copies.1)?));
    let copied = copied.map_err(|e| format!("can't copy the pack and the content to {}: {e}", scratch.display()))?;
    println!("{name}: {copied} files of its pack and the content copied to {}", scratch.display());

    // The game, a part at a time.
    let options = Options { packs_dir: None, packs: vec![copies.0.clone()], content: Some(copies.1.clone()), ..Options::default() };
    let mut loading = Loading::new(&name, options);
    let started = Instant::now();
    while let Some(part) = loading.next() {
        let t = Instant::now();
        loading.step().map_err(|e| format!("{}\n{e}", e.report))?;
        println!("  loaded {part:?} in {:.1?} (the bar at {:.0}%)", t.elapsed(), 100.0 * loading.fraction());
    }
    println!("  the game loaded in {:.1?}", started.elapsed());
    let game = loading.finish().ok_or("the loading didn't finish")?;
    let content = game.game.content.clone();

    // The thread's runtime: the wait a match's first tick had, before.
    let t = Instant::now();
    let loaded = preload::runtime(&content)?;
    println!("  this thread's runtime loaded in {:.1?} (loaded now: {loaded})", t.elapsed());
    let t = Instant::now();
    preload::runtime(&content)?;
    println!("  and asked for again in {:.1?}", t.elapsed());

    // A match of it, made ready a step at a time.
    let m = nettai_match::pick::live(&content, &name, seed, None)?;
    let set = nettai_match::Set::of(&content, &m, seed);
    let mut preload = Preload::new(&game, &set, 0, size);
    let (mut steps, mut longest, started) = (0, Duration::ZERO, Instant::now());
    loop {
        let t = Instant::now();
        let more = preload.step();
        longest = longest.max(t.elapsed());
        steps += 1;
        if !more {
            break;
        }
    }
    let fractions = [Part::Match, Part::Side(0), Part::Side(1)].map(|p| preload.fraction_of(p));
    println!("  the match made ready in {steps} steps and {:.1?} (the longest {longest:.1?}); the parts at {fractions:?}", started.elapsed());
    if let Some(why) = preload.stopped() {
        return Err(format!("the match can't start: {why}"));
    }
    let missing = preload.missing();
    println!("  what the pack hasn't of it: {}", if missing.is_empty() { "nothing".to_string() } else { format!("{} things:\n    {}", missing.len(), missing.join("\n    ")) });

    // The pack and the content go; the match is played without them.
    std::fs::remove_dir_all(&scratch).map_err(|e| format!("can't delete {}: {e}", scratch.display()))?;
    if copies.0.exists() || copies.1.exists() {
        return Err("the copies are still there".into());
    }
    let mut player = preload.player(driver(&set));
    let mut plain = Player::new(&game, driver(&set));
    let started = Instant::now();
    let (tick, picture) = play(&mut player, &mut plain, ticks, size)?;
    let late = player.late();
    println!(
        "  {} ticks played with the pack and the content deleted, in {:.1?}, each picture and sound the same as a player's made without the preload: \
         the longest tick {tick:.1?}, the longest picture ({}x{}) {picture:.1?}; {}",
        player.ticks(),
        started.elapsed(),
        size.0,
        size.1,
        player.position()
    );
    println!(
        "  made late: {} runtimes loaded, {} parts read, {} strings laid out and {} glyphs made for the first time",
        late.runtimes, late.reads, late.layouts, late.glyphs
    );
    if let Some(why) = player.diverged() {
        return Err(why.to_string());
    }
    if late.runtimes != 0 || late.reads != 0 {
        return Err(format!("the preloaded match loaded {} runtimes and read {} parts on the way", late.runtimes, late.reads));
    }

    // The same match on a thread that preloaded nothing: what its first
    // battle waits for (the files are gone: it comes from memory too).
    let (made, late) = std::thread::scope(|s| {
        s.spawn(|| {
            let t = Instant::now();
            let player = Player::new(&game, driver(&set));
            (t.elapsed(), player.late())
        })
        .join()
        .expect("the thread ran")
    });
    println!("  without the preload, on a thread of its own: the player and its first battle took {made:.1?} ({} runtime loaded late)", late.runtimes);
    if late.runtimes != 1 {
        return Err(format!("a thread that preloaded nothing loaded {} runtimes for its first battle", late.runtimes));
    }
    Ok(())
}

fn main() {
    if let Err(why) = run() {
        eprintln!("preload: {why}");
        std::process::exit(1);
    }
}
