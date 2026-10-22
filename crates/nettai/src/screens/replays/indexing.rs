//! The replay's index, made on a thread of its own (docs/app.md §16): a second `Viewer` of the replay watched, with
//! its sound (made and never heard, so that the states kept hold it), plays it through from its start there and sends
//! each part of the index as it is made (`Viewer::index_part`: the states kept to seek from, and how far it reached).
//! The viewer watched takes them in each frame (`Viewer::take_index`), and its index, the bar's fill and the edge no
//! seek goes past go on with them. The replay plays the same on both, so the states are the ones the viewer watched
//! would make itself (docs/frontend.md §8, "Seeking").
//!
//! **What it costs.** The window's thread spends nothing on it but taking the parts in. The thread indexes as fast
//! as a viewer indexes itself, a tick with both consoles' sound at a time: about 10,000 ticks a second in the `dev`
//! build (five minutes in 1.8 s), 14,000 in a release build, where a slice a frame on the window's thread went
//! 1,300 while the replay played (five minutes in 14 s). It keeps none of the states it sends: the viewer watched
//! has them, as it had them before. It ends at the replay's end, or when the screen drops its end of the channel
//! (the replay is left), at its next part.

use nettai_frontend::game::Loaded;
use nettai_frontend::viewer::{IndexPart, KEYFRAME_EVERY, Viewer};
use nettai_platform::time::Instant;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

/// The ticks of a part: one stretch between two kept states (a part a few hundredths of a second of the thread's).
const PART: u32 = KEYFRAME_EVERY;

/// The thread's index, while it is made.
pub struct Indexing {
    parts: Receiver<IndexPart>,
    /// When it was started, and whether a part has come.
    pub began: Instant,
    pub heard: bool,
}

/// What came of the thread in a frame.
pub enum Taken {
    /// Parts, or none yet: the index goes on.
    Going,
    /// The last part: the whole replay is indexed.
    Whole,
    /// The thread ended short of the replay's end (it couldn't read or open the replay, or the engine stopped it).
    Ended,
}

impl Indexing {
    /// Start the thread for the replay at `path`, played on `game` (with its sound). None if it couldn't start.
    pub fn start(game: Loaded, path: PathBuf) -> Option<Indexing> {
        let (send, parts) = mpsc::channel();
        match nettai_platform::thread::spawn("index", move || work(game, path, send)) {
            Ok(()) => Some(Indexing { parts, began: Instant::now(), heard: false }),
            Err(why) => {
                crate::errors::log("starting the replay's index", &why);
                None
            }
        }
    }

    /// The parts `parts` hands over, as a thread's (the tests make them).
    #[cfg(test)]
    pub fn of(parts: Receiver<IndexPart>) -> Indexing {
        Indexing { parts, began: Instant::now(), heard: false }
    }

    /// Take the parts made since into `viewer`, in order.
    pub fn take(&mut self, viewer: &mut Viewer) -> Taken {
        loop {
            match self.parts.try_recv() {
                Ok(part) => {
                    self.heard = true;
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

/// The thread: a viewer of the replay, its sound made and not heard, playing it through a part at a time, each sent
/// as it is made. It ends at the replay's end, when the screen drops its end of the channel, or when the engine stops
/// it outside a tick (a tick's own stop is where the replay ends: its last part is whole).
fn work(mut game: Loaded, path: PathBuf, parts: Sender<IndexPart>) {
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
        let part = match nettai_frontend::session::in_engine(|| viewer.index_part(PART)) {
            Ok(part) => part,
            Err(panic) => {
                let at = viewer.position();
                return crate::errors::log(&doing, &format_args!("the engine stopped at tick {at}: {}", nettai_frontend::session::panic_message(panic.as_ref())));
            }
        };
        let whole = part.whole();
        if parts.send(part).is_err() || whole {
            return;
        }
    }
}
