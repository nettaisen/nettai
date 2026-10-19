//! Seeking in a replay, checked and measured on a game's real packs.
//!
//! It records a seeded random match (both navis mashing; the custom
//! screens picked as the stand-in picks), then:
//!
//! - plays the replay straight from each console with a [`Player`] (which
//!   has no seeking in it) and notes, for every tick, the battle's digest,
//!   the picture as presented at twice the frame (text and all), the HUD's
//!   state and the tick's sound;
//! - opens a [`Viewer`] and seeks about it, a few hundred seeded seeks of
//!   every kind (far and near, back and on, a frame back again and again,
//!   to each round's start, changing console between them), and after each
//!   checks the battle, the picture and the HUD against straight playback
//!   at that tick, and the sound of the ticks that follow, sample for
//!   sample;
//! - measures what seeking costs: the states kept and the memory they
//!   take (a battle's snapshot and a console's sound, by counting what
//!   copying each allocates), the time to index the whole replay, the
//!   first far seek of a viewer that hasn't, and a seek and a frame back
//!   in an indexed one, with sound and without. The times are of the
//!   processor (the C library's `clock`), not of the wall, so that a
//!   machine busy with something else says the same.
//!
//! ```text
//! cargo run --release -p nettai-frontend --example seek -- --game exe6 --seed 7 [--ticks 18000] [--save set.nettaireplay]
//! ```
//!
//! `--ticks` stops the recording after that many (default 18000: five
//! minutes), `--save` keeps the replay (for `nettai-tool --replay`). The
//! packs are found as the program finds them (`$NETTAI_PACKS`).

use nettai_battle::battle::mode;
use nettai_frontend::Player;
use nettai_frontend::driver::{InFight, LivePlayer, OnCustomScreen, Opponent, bot_buttons};
use nettai_frontend::game::{self, Loaded, Options};
use nettai_frontend::player::time_of;
use nettai_frontend::replay::{Info, MatchKind, Recorder, Replay, ReplayPlayer};
use nettai_frontend::session::Session;
use nettai_frontend::viewer::{KEYFRAME_EVERY, Viewer};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

unsafe extern "C" {
    /// The C library's `clock`: the processor time this process has used,
    /// in millionths of a second (POSIX's `CLOCKS_PER_SEC`).
    fn clock() -> std::ffi::c_ulong;
}

/// The processor time this process has used.
fn used() -> Duration {
    // SAFETY: `clock` takes nothing and touches nothing of ours.
    Duration::from_micros(unsafe { clock() } as u64)
}

/// A moment of processor time, to measure from.
#[derive(Clone, Copy)]
struct Instant(Duration);

impl Instant {
    fn now() -> Instant {
        Instant(used())
    }

    fn elapsed(&self) -> Duration {
        used().saturating_sub(self.0)
    }
}

/// The system's allocator, counting the bytes held: what a copy of a
/// battle or of a console's sound takes is the difference across making
/// it.
struct Counting;

static HELD: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is the system allocator's own, with the same
// arguments; the count beside it touches no memory of theirs.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HELD.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, at: *mut u8, layout: Layout) {
        HELD.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(at, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// A quick hash of words (FNV-1a's steps over each).
fn hash(words: impl Iterator<Item = u64>) -> u64 {
    words.fold(0xCBF2_9CE4_8422_2325, |h, w| (h ^ w).wrapping_mul(0x0000_0100_0000_01B3))
}

/// What straight playback shows and makes at a tick, each as a hash.
#[derive(Clone, Copy, PartialEq, Eq)]
struct At {
    digest: u64,
    picture: u64,
    hud: u64,
    /// Of the tick that led here.
    sound: u64,
}

const SIZE: (usize, usize) = (480, 320);

fn sound_hash(samples: &[[f32; 2]]) -> u64 {
    hash(samples.iter().map(|s| (s[0].to_bits() as u64) << 32 | s[1].to_bits() as u64).chain([samples.len() as u64]))
}

/// The replay played straight from `view`'s console by a player: what it
/// is at every tick, from before the first.
fn straight(game: &Loaded, replay: &Replay, view: u8) -> Result<Vec<At>, String> {
    let driver = ReplayPlayer::new(&game.game.content, replay, Some(view)).map_err(|e| e.to_string())?;
    let mut p = Player::new(game, Box::new(driver));
    let mut buffer = vec![0u32; SIZE.0 * SIZE.1];
    let mut out = Vec::new();
    let mut sound = Vec::new();
    loop {
        sound.clear();
        p.take_samples(&mut sound);
        p.present(&mut buffer, SIZE.0, SIZE.1);
        out.push(At {
            digest: p.battle().digest(),
            picture: hash(buffer.iter().map(|&c| c as u64)),
            hud: hash(format!("{:?}", p.renderer().hud_state).bytes().map(u64::from)),
            sound: sound_hash(&sound),
        });
        if !p.tick(0) {
            break;
        }
    }
    if let Some(differs) = p.diverged() {
        return Err(format!("the replay doesn't reproduce: {differs}"));
    }
    Ok(out)
}

/// That `v` is as straight playback is at the tick it is at.
fn same(v: &mut Viewer, straight: &[At], buffer: &mut [u32], what: &str) -> Result<(), String> {
    let at = straight[v.position() as usize];
    let tick = v.position();
    if v.battle().digest() != at.digest {
        return Err(format!("{what}: the battle at tick {tick} isn't straight playback's"));
    }
    v.present(buffer, SIZE.0, SIZE.1);
    if hash(buffer.iter().map(|&c| c as u64)) != at.picture {
        return Err(format!("{what}: the picture at tick {tick} isn't straight playback's"));
    }
    if hash(format!("{:?}", v.renderer().hud_state).bytes().map(u64::from)) != at.hud {
        return Err(format!("{what}: the HUD at tick {tick} isn't straight playback's"));
    }
    Ok(())
}

/// Play `v` on `ticks` ticks, each as straight playback has it, its sound
/// included.
fn follow(v: &mut Viewer, straight: &[At], buffer: &mut [u32], ticks: u32, what: &str) -> Result<(), String> {
    let mut sound = Vec::new();
    v.take_samples(&mut sound);
    if !sound.is_empty() {
        return Err(format!("{what}: {} samples left over from before", sound.len()));
    }
    for _ in 0..ticks {
        if !v.step_forward() {
            break;
        }
        same(v, straight, buffer, what)?;
        sound.clear();
        v.take_samples(&mut sound);
        if sound_hash(&sound) != straight[v.position() as usize].sound {
            return Err(format!("{what}: the sound of tick {} isn't straight playback's", v.position() - 1));
        }
    }
    Ok(())
}

/// SplitMix64: the seeks' random numbers.
struct Random(u64);

impl Random {
    fn below(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n.max(1) as u64) as u32
    }
}

fn run() -> Result<String, String> {
    let (mut name, mut seed, mut ticks, mut save) = (None, None, 18_000u64, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--game" => name = Some(value()?),
            "--seed" => seed = Some(value()?.parse::<u32>().map_err(|e| format!("--seed: {e}"))?),
            "--ticks" => ticks = value()?.parse().map_err(|e| format!("--ticks: {e}"))?,
            "--save" => save = Some(std::path::PathBuf::from(value()?)),
            other => return Err(format!("no option {other:?} (--game GAME --seed N [--ticks N] [--save FILE])")),
        }
    }
    let name = name.ok_or("say which game: --game exe6, exe5 or exe4")?;
    let seed = seed.ok_or("say the match's seed: --seed N")?;
    let game = game::load(&name, &Options::default()).map_err(|e| format!("{}\n{e}", e.report))?;
    let content = &game.game.content;

    // The set, recorded: both navis mashing.
    let m = nettai_match::pick::live(content, &name, seed, None)?;
    let m = nettai_match::Match { seed: Some(seed), ..m };
    let path = save.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("nettai-seek-{}.nettaireplay", std::process::id())));
    let file = std::fs::File::create(&path).map_err(|e| format!("can't write {}: {e}", path.display()))?;
    let opponent = Opponent { in_fight: InFight::AttacksAtRandom, on_custom_screen: OnCustomScreen::PicksFirst };
    let driver = LivePlayer::against(nettai_match::Set::of(content, &m, seed), opponent, seed as u64);
    let mut s = Session::new(Box::new(driver));
    s.record(Recorder::new(Box::new(std::io::BufWriter::new(file)), content, &m, &Info::of(MatchKind::Offline, 0))?)?;
    let mut masher = nettai_netplay_masher(seed as u64);
    while s.ticks < ticks {
        let mashed = masher.buttons();
        let keys = if s.battle.round.mode == mode::CUSTOM { bot_buttons(&s.battle, 0, s.ticks as u32) } else { mashed };
        if !s.step(keys) {
            break;
        }
    }
    drop(s);
    let bytes = std::fs::read(&path).map_err(|e| format!("can't read {}: {e}", path.display()))?;
    if save.is_none() {
        let _ = std::fs::remove_file(&path);
    }
    let replay = Replay::read(&bytes).map_err(|e| e.to_string())?;
    let length = replay.ticks.len() as u32;
    let rounds = replay.round_starts();
    if length < 4 * KEYFRAME_EVERY {
        return Err(format!("the set is only {length} ticks: nothing to seek about in"));
    }

    // Straight playback, from each console.
    let started = Instant::now();
    let sides = [straight(&game, &replay, 0)?, straight(&game, &replay, 1)?];
    let played = started.elapsed() / 2;
    let differ = sides[0].iter().zip(&sides[1]).filter(|(a, b)| a.picture != b.picture).count();
    if differ == 0 {
        return Err("the two consoles show alike: nothing to tell a change of console by".into());
    }

    // A viewer that hasn't indexed: its first seek to the end.
    let mut buffer = vec![0u32; SIZE.0 * SIZE.1];
    let mut fresh = Viewer::open(&game, &replay, Some(0)).map_err(|e| e.to_string())?;
    let started = Instant::now();
    fresh.seek(length);
    let first_far = started.elapsed();
    same(&mut fresh, &sides[0], &mut buffer, "a first seek to the end")?;
    drop(fresh);

    // The same without sound: what of a tick is the battle's and the
    // HUDs', and what the two consoles' sound.
    let mut silent = Viewer::with(&game.game.content, &replay, Some(0), game.renderer(), None, None).map_err(|e| e.to_string())?;
    let started = Instant::now();
    silent.seek(length);
    let silent_far = started.elapsed();
    drop(silent);

    // Indexing the whole replay, a slice at a time: the whole's time, and
    // the best slice's a tick.
    let mut v = Viewer::open(&game, &replay, Some(0)).map_err(|e| e.to_string())?;
    const SLICE: u32 = 1000;
    let started = Instant::now();
    while !v.index(SLICE) {}
    let indexing = started.elapsed();
    let kept = v.kept();
    same(&mut v, &sides[0], &mut buffer, "after indexing")?;

    // What a kept state takes: a battle's snapshot, each console's sound
    // and each console's HUD state (which is all in place).
    let held = || HELD.load(Ordering::Relaxed);
    let before = held();
    let snapshot = v.battle().save_state();
    let battle_bytes = held() - before;
    let sound_bytes = game.sound.as_ref().map_or(0, |s| {
        let sound = nettai_audio::BattleAudio::with_banks(s.banks.clone(), s.songs.clone());
        let before = held();
        let copy = sound.clone();
        let bytes = held() - before;
        drop(copy);
        bytes
    });
    drop(snapshot);
    let state_bytes = battle_bytes + 2 * sound_bytes + 2 * std::mem::size_of::<nettai_render::hud::HudState>();

    // Seeks of every kind, each checked.
    let mut random = Random(seed as u64);
    let mut view = 0u8;
    let mut took: Vec<Duration> = Vec::new();
    let mut timed = |v: &mut Viewer, to: u32| {
        let started = Instant::now();
        v.seek(to);
        took.push(started.elapsed());
    };
    for n in 0..300 {
        let here = v.position();
        let (to, what) = match n % 6 {
            0 => (random.below(length + 1), "a seek anywhere"),
            1 => (here.saturating_sub(1 + random.below(600)), "a seek back"),
            2 => ((here + 1 + random.below(600)).min(length), "a seek on"),
            3 => (rounds[random.below(rounds.len() as u32) as usize], "a seek to a round's start"),
            4 => ((random.below(length / KEYFRAME_EVERY) * KEYFRAME_EVERY + random.below(3)).saturating_sub(1), "a seek beside a kept state"),
            _ => (if random.below(8) == 0 { length } else { random.below(length + 1) }, "a seek after a change of console"),
        };
        if n % 6 == 5 {
            view ^= 1;
            v.set_view(view);
        }
        timed(&mut v, to);
        let what = format!("{what} (to tick {to} from {here}, side {view})");
        same(&mut v, &sides[view as usize], &mut buffer, &what)?;
        follow(&mut v, &sides[view as usize], &mut buffer, 4, &what)?;
    }
    // A frame back, again and again.
    v.set_view(0);
    v.seek(length / 2 + KEYFRAME_EVERY / 2);
    let steps = 2 * KEYFRAME_EVERY;
    let mut stepping: Vec<Duration> = (0..steps)
        .map(|_| {
            let started = Instant::now();
            v.step_back();
            started.elapsed()
        })
        .collect();
    same(&mut v, &sides[0], &mut buffer, "frames back")?;
    follow(&mut v, &sides[0], &mut buffer, 30, "frames on after frames back")?;
    if let Some(differs) = v.diverged() {
        return Err(format!("the replay doesn't reproduce: {differs}"));
    }

    let seconds = time_of(length as u64).as_secs_f64();
    took.sort();
    stepping.sort();
    let (seeks, median, worst) = (took.len(), took[took.len() / 2], took[took.len() - 1]);
    let (step, worst_step) = (stepping[stepping.len() / 2], stepping[stepping.len() - 1]);
    Ok(format!(
        "{name} seed {seed}: a set of {length} ticks ({:.0} s, {} rounds), kept{}\n\
         every seek lands where straight playback is: {seeks} seeks and {steps} frames back, the battle's digest, the picture at \
         {}x{} with its text, the HUD's state and the sound of the ticks after, from both consoles ({differ} of their pictures differ)\n\
         states kept: {kept} (one every {KEYFRAME_EVERY} ticks and at each round's start), {:.1} MB ({:.0} KB each: a battle's snapshot {:.0} KB, each console's sound {:.0} KB)\n\
         indexing the whole replay, in slices of {SLICE} ticks: {indexing:.2?} of processor time ({:.1} us a tick; {:.2} s for five minutes); \
         playing it straight with its picture: {played:.2?}\n\
         a first seek to the end, nothing indexed: {first_far:.2?}; without sound, {silent_far:.2?} ({:.1} us a tick)\n\
         a seek in the indexed replay: {median:.2?} (the median; {worst:.2?} at worst); a frame back: {step:.2?} (the median; {worst_step:.2?} at worst)",
        seconds,
        rounds.len(),
        save.map_or(String::new(), |p| format!(" as {}", p.display())),
        SIZE.0,
        SIZE.1,
        (state_bytes * kept) as f64 / 1e6,
        state_bytes as f64 / 1e3,
        battle_bytes as f64 / 1e3,
        sound_bytes as f64 / 1e3,
        indexing.as_secs_f64() * 1e6 / length as f64,
        indexing.as_secs_f64() / length as f64 * 17_918.0,
        silent_far.as_secs_f64() * 1e6 / length as f64,
    ))
}

/// The left navi's presses: a masher that shoots too.
fn nettai_netplay_masher(seed: u64) -> nettai_netplay::standin::Masher {
    let mut m = nettai_netplay::standin::Masher::new(seed ^ 0x5EED);
    m.buster = true;
    m
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
