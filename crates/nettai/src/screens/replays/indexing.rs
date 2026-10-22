//! The replay's index, made on threads of their own (docs/app.md §16): the battle's and one a sound, the parts of the
//! index going from one to the next as they are made. A second `Viewer` of the replay watched, silent, plays the
//! battle through from its start on the first (`Viewer::index_battle`: the states kept to seek from, without their
//! sound, and each tick's sound cues for each of the viewer's sounds, `SOUNDS`: each console's and each side's own
//! effects); each sound plays those cues on one of its own (`ConsoleSound::play`, made and never heard, so that the
//! states kept hold it) and puts itself into the part's states; the last sends the parts on, whole
//! (`BattlePart::into_part`). The viewer watched takes them in each frame (`Viewer::take_index`),
//! and its index, the bar's fill and the edge no seek goes past go on with them. The replay plays the same anywhere,
//! so the states are the ones the viewer watched would make itself (docs/frontend.md §8, "Seeking").
//!
//! **What it costs.** The window's thread spends nothing on it but taking the parts in. Half of a tick or more was
//! the two consoles' sound (the user: "the preload still seems slower than it should be?"; approving "run each
//! player's sound on its own thread beside the game's thread"): the battle's thread now plays the battle and the
//! HUDs alone, and the sound threads go beside it, so the index goes as fast as the slowest of the three. On the
//! user's 19:42 EXE5 replay in a release build (the battle the slowest): 70,623 ticks in 3.8 to 4.0 s, about 18,000
//! a second, where one thread making the sound too took 7.1 to 7.4 s, about 9,700 a second (docs/frontend.md §8,
//! `examples/index.rs`). None keeps the states it sends: the viewer watched has them, as it had them before.
//! Between two threads at most `AHEAD` parts wait; the window's thread takes every part sent each frame (its
//! channel is unbounded, so it never waits on a lock a thread holds). Each thread ends at the replay's end, at its
//! next part once the one after it has ended (the screen dropped its end of the channel: the replay is left), or
//! when the one before it ends; one that ends short of the end ends the index where it got.

use nettai_frontend::game::Loaded;
use nettai_frontend::viewer::{BattlePart, ConsoleSound, IndexPart, KEYFRAME_EVERY, Viewer};
use nettai_platform::time::Instant;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// The ticks of a part: one stretch between two kept states (a part a few hundredths of a second of each thread's).
const PART: u32 = KEYFRAME_EVERY;

/// The parts that wait between two of the threads at most: a thread ahead of the next waits for it.
const AHEAD: usize = 4;

/// The threads' index, while it is made.
pub struct Indexing {
    parts: Receiver<IndexPart>,
    /// When it was started.
    pub began: Instant,
}

/// What came of the threads in a frame.
pub enum Taken {
    /// Parts, or none yet: the index goes on.
    Going,
    /// The last part: the whole replay is indexed.
    Whole,
    /// The threads ended short of the replay's end (they couldn't read or open the replay, or the engine or the sound
    /// stopped them): the index stays where it got.
    Ended,
}

impl Indexing {
    /// Start the threads for the replay at `path`, played on `game` (each console's sound on a thread of its own,
    /// where the game has sound). None if they couldn't start.
    pub fn start(mut game: Loaded, path: PathBuf) -> Option<Indexing> {
        let (send, parts) = mpsc::channel();
        let sounds = ConsoleSound::of(&game);
        // (The battle's thread plays it silent: the sound is the other threads'.)
        game.sound = None;
        let started = match sounds {
            Some(sounds) => {
                let doing = format!("indexing the sound of {}", path.display());
                // (Each sound's thread hands its parts to the next's, the last sends them on whole. The last first:
                // where one doesn't start, those started have no thread before them, and end.)
                let mut next: Box<dyn FnMut(BattlePart) -> bool + Send> = Box::new(move |part| send.send(part.into_part()).is_ok());
                let mut started = Ok(());
                for (k, console) in sounds.into_iter().enumerate().rev() {
                    let (to, gets) = mpsc::sync_channel(AHEAD);
                    let on = std::mem::replace(&mut next, Box::new(move |part| to.send(part).is_ok()));
                    let doing = doing.clone();
                    started = started.and_then(|()| nettai_platform::thread::spawn(&format!("index sound {}", k + 1), move || sound(console, gets, &doing, on)));
                }
                started.and_then(|()| nettai_platform::thread::spawn("index", move || battle(game, path, next)))
            }
            None => nettai_platform::thread::spawn("index", move || battle(game, path, |part| send.send(part.into_part()).is_ok())),
        };
        match started {
            Ok(()) => Some(Indexing { parts, began: Instant::now() }),
            Err(why) => {
                crate::errors::log("starting the replay's index", &why);
                None
            }
        }
    }

    /// The parts `parts` hands over, as the threads' (the tests make them).
    #[cfg(test)]
    pub fn of(parts: Receiver<IndexPart>) -> Indexing {
        Indexing { parts, began: Instant::now() }
    }

    /// Take the parts made since into `viewer`, in order.
    pub fn take(&mut self, viewer: &mut Viewer) -> Taken {
        loop {
            match self.parts.try_recv() {
                Ok(part) => {
                    let whole = part.whole();
                    viewer.take_index(part);
                    if whole {
                        return Taken::Whole;
                    }
                }
                Err(TryRecvError::Empty) => return Taken::Going,
                Err(TryRecvError::Disconnected) => return Taken::Ended,
            }
        }
    }
}

/// The battle's thread: a silent viewer of the replay, playing it through a part at a time, each handed `on` as it
/// is made (false: the next thread has ended). It ends at the replay's end, when the next thread has ended, or when
/// the engine stops it outside a tick (a tick's own stop is where the replay ends: its last part is whole).
fn battle(mut game: Loaded, path: PathBuf, mut on: impl FnMut(BattlePart) -> bool) {
    let doing = format!("indexing {}", path.display());
    // (Nothing is drawn: no text to lay out.)
    game.font = None;
    let replay = match crate::replays::read(&path) {
        Ok(replay) => replay,
        Err(why) => return crate::errors::log(&doing, &why),
    };
    let mut viewer = match nettai_frontend::session::in_engine(|| Viewer::open(&game, &replay, None)) {
        Ok(Ok(viewer)) => viewer,
        Ok(Err(refused)) => return crate::errors::log(&doing, &refused),
        Err(panic) => return crate::errors::log(&doing, &format_args!("the engine stopped: {}", nettai_frontend::session::panic_message(panic.as_ref()))),
    };
    loop {
        let part = match nettai_frontend::session::in_engine(|| viewer.index_battle(PART)) {
            Ok(part) => part,
            Err(panic) => {
                let at = viewer.position();
                return crate::errors::log(&doing, &format_args!("the engine stopped at tick {at}: {}", nettai_frontend::session::panic_message(panic.as_ref())));
            }
        };
        let whole = part.whole();
        if !on(part) || whole {
            return;
        }
    }
}

/// A console's sound thread: the parts from the thread before, each played and handed `on` (false: the next thread
/// has ended). It ends when the thread before has ended and its parts are played, or when the next thread has
/// ended.
fn sound(mut console: ConsoleSound, parts: Receiver<BattlePart>, doing: &str, mut on: impl FnMut(BattlePart) -> bool) {
    for mut part in parts {
        if let Err(panic) = nettai_frontend::session::in_engine(|| console.play(&mut part)) {
            let at = part.reached();
            return crate::errors::log(doing, &format_args!("the sound stopped by tick {at}: {}", nettai_frontend::session::panic_message(panic.as_ref())));
        }
        if !on(part) {
            return;
        }
    }
}
