//! A replay's index made on threads, measured on a game's real packs: on
//! one thread, a viewer making its own sound as it goes
//! ([`Viewer::index_part`]), and on several, wired as the app wires them
//! (crates/nettai/src/screens/replays/indexing.rs): the battle's on one (a
//! silent viewer, [`Viewer::index_battle`]) and each of the viewer's sounds
//! on one of its own ([`ConsoleSound::play`]), the parts going from one to
//! the next as they are made. Each index is taken in by a viewer here as it
//! comes, as the app's window takes it in. It says the wall's time for each
//! (opening the thread's viewer, then indexing) and the ticks a second,
//! and the battle's share alone (a silent viewer), which the threads go at
//! most as fast as.
//!
//! Then it checks that the two made the same index: at every state kept,
//! each viewer sought there is the same battle, and both consoles' sound
//! over the half second after is the same, sample for sample.
//!
//! ```text
//! cargo run --release -p nettai-frontend --example index -- set.nettaireplay
//! ```
//!
//! The replay's game is loaded with its sound, extracted from its ROMs as the
//! tools find them (`$NETTAI_ROMS`).

use nettai_frontend::game::{self, Loaded, Options};
use nettai_frontend::replay::Replay;
use nettai_frontend::viewer::{BattlePart, ConsoleSound, IndexPart, KEYFRAME_EVERY, Viewer};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// The parts' ticks, as the app's.
const PART: u32 = KEYFRAME_EVERY;

/// What making an index took: opening the viewer that makes it, and the
/// index after that, to its last part taken in.
struct Took {
    open: Duration,
    index: Duration,
}

/// The index made on one thread, a viewer with its sound playing it
/// through; taken in by `v`.
fn on_one(game: &Loaded, replay: &Replay, v: &mut Viewer) -> Result<Took, String> {
    let (send, parts) = mpsc::channel::<IndexPart>();
    let began = Instant::now();
    let open = std::thread::scope(|scope| {
        let thread = scope.spawn(move || -> Result<Duration, String> {
            let mut other = Viewer::open(game, replay, None).map_err(|e| e.to_string())?;
            let open = began.elapsed();
            loop {
                let part = other.index_part(PART);
                let whole = part.whole();
                if send.send(part).is_err() || whole {
                    return Ok(open);
                }
            }
        });
        for part in parts.iter() {
            v.take_index(part);
        }
        thread.join().map_err(|_| "the thread stopped".to_string())?
    })?;
    Ok(Took { open, index: began.elapsed() - open })
}

/// The index made on threads: a silent viewer playing the battle through,
/// each of the viewer's sounds on a thread of its own; taken in by `v`.
fn on_three(game: &Loaded, replay: &Replay, v: &mut Viewer) -> Result<Took, String> {
    let (send, parts) = mpsc::channel::<IndexPart>();
    let began = Instant::now();
    let sounds = ConsoleSound::of(game).ok_or("the game's sound isn't loaded")?;
    let open = std::thread::scope(|scope| {
        // (Each sound's thread hands its parts to the next's; the last sends them on whole.)
        let (to_first, mut gets) = mpsc::sync_channel::<BattlePart>(4);
        let count = sounds.len();
        for (k, mut sound) in sounds.into_iter().enumerate() {
            let (to_next, next_gets) = mpsc::sync_channel::<BattlePart>(4);
            let from = std::mem::replace(&mut gets, next_gets);
            let send = send.clone();
            scope.spawn(move || {
                for mut part in from {
                    sound.play(&mut part);
                    let sent = if k + 1 == count { send.send(part.into_part()).is_ok() } else { to_next.send(part).is_ok() };
                    if !sent {
                        return;
                    }
                }
            });
        }
        drop((send, gets));
        let thread = scope.spawn(move || -> Result<Duration, String> {
            let mut silent = Viewer::with(&game.game.content, replay, None, game.renderer(), None, None).map_err(|e| e.to_string())?;
            let open = began.elapsed();
            loop {
                let part = silent.index_battle(PART);
                let whole = part.whole();
                if to_first.send(part).is_err() || whole {
                    return Ok(open);
                }
            }
        });
        for part in parts.iter() {
            v.take_index(part);
        }
        thread.join().map_err(|_| "the thread stopped".to_string())?
    })?;
    Ok(Took { open, index: began.elapsed() - open })
}

/// The battle's share alone, on this thread: a silent viewer playing the
/// replay through, its parts thrown away (what the threads go at most as
/// fast as).
fn battle_alone(game: &Loaded, replay: &Replay) -> Result<Duration, String> {
    let mut silent = Viewer::with(&game.game.content, replay, None, game.renderer(), None, None).map_err(|e| e.to_string())?;
    let began = Instant::now();
    while !silent.index_battle(PART).whole() {}
    Ok(began.elapsed())
}

/// The ticks the index keeps a state at: every [`KEYFRAME_EVERY`]th and
/// each round's first, up to the end.
fn kept(v: &Viewer) -> Vec<u32> {
    let mut ticks: Vec<u32> = (0..=v.length()).step_by(KEYFRAME_EVERY as usize).chain(v.rounds().iter().copied()).collect();
    ticks.sort();
    ticks.dedup();
    ticks
}

fn run() -> Result<String, String> {
    let path = std::env::args().nth(1).ok_or("say which replay: index FILE")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("can't read {path}: {e}"))?;
    let replay = Replay::read(&bytes).map_err(|e| e.to_string())?;
    let name = replay.head.game.clone();
    let game = game::load(&name, &Options::default()).map_err(|e| format!("{}\n{e}", e.report))?;
    let length = replay.ticks.len() as u32;

    let mut one = Viewer::open(&game, &replay, None).map_err(|e| e.to_string())?;
    let by_one = on_one(&game, &replay, &mut one)?;
    let mut three = Viewer::open(&game, &replay, None).map_err(|e| e.to_string())?;
    let by_three = on_three(&game, &replay, &mut three)?;
    let alone = battle_alone(&game, &replay)?;
    for (v, how) in [(&one, "one thread"), (&three, "three threads")] {
        if v.indexed() != length {
            return Err(format!("the index on {how} reached {} of {length} ticks", v.indexed()));
        }
    }

    // The same index: at every state kept, the same battle and the same
    // sound after it, both consoles'.
    let at = kept(&one);
    let after = Instant::now();
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let mut sounding = 0;
    for &tick in &at {
        one.seek(tick);
        three.seek(tick);
        if one.battle().digest() != three.battle().digest() {
            return Err(format!("the battle at tick {tick} isn't the same"));
        }
        for _ in 0..30 {
            if !(one.step_forward() & three.step_forward()) {
                break;
            }
        }
        for side in 0..2 {
            a.clear();
            b.clear();
            one.take_samples_of(side, &mut a);
            three.take_samples_of(side, &mut b);
            if a != b {
                return Err(format!("side {side}'s sound after tick {tick} isn't the same"));
            }
            sounding += a.iter().any(|s| s[0] != 0.0) as usize;
        }
    }
    if one.sought() > 0 || three.sought() > 0 {
        return Err("a seek to a state kept played ticks: a state is missing".into());
    }
    let checked = after.elapsed();

    let rate = |took: &Took| length as f64 / took.index.as_secs_f64();
    let per = |took: &Took| took.index.as_secs_f64() * 1e6 / length as f64;
    Ok(format!(
        "{path}: {name}, {length} ticks ({:.0} s, {} rounds)\n\
         on one thread (a viewer making its own sound): opened in {:.2?}, indexed in {:.2?} of the wall: {:.0} ticks a second, {:.1} us a tick\n\
         on three threads (the battle; each console's sound): opened in {:.2?}, indexed in {:.2?} of the wall: {:.0} ticks a second, {:.1} us a tick ({:.2}x)\n\
         the battle's share alone (a silent viewer, its parts thrown away): {alone:.2?}, {:.1} us a tick\n\
         the same index: {} states kept, each the same battle and the same sound over the half second after it, both consoles' \
         ({sounding} of {} sounding), checked in {checked:.2?}",
        nettai_frontend::player::time_of(length as u64).as_secs_f64(),
        one.rounds().len(),
        by_one.open,
        by_one.index,
        rate(&by_one),
        per(&by_one),
        by_three.open,
        by_three.index,
        rate(&by_three),
        per(&by_three),
        rate(&by_three) / rate(&by_one),
        alone.as_secs_f64() * 1e6 / length as f64,
        at.len(),
        2 * at.len(),
    ))
}

fn main() {
    nettai_frontend::session::quiet_engine_panics();
    match run() {
        Ok(said) => println!("{said}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
