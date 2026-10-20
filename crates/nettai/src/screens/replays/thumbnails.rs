//! The bar's previews (docs/design/app-ui.md §2.96; docs/app.md §16): while the pointer is over the viewer's bar, a
//! small picture of the battle at the time under it, above the bar, with the time under it.
//!
//! **A second viewer of its own.** The previews are drawn by a silent `Viewer` of the same replay on a thread of
//! its own, opened at the first hover: the replay played again from its file, the game's graphics as the window
//! shows them (the language, the graphics mode, the text), no sound. It indexes the replay ahead as the window's
//! does, then seeks to each time asked for and presents that console's picture (`Viewer::present_view`) at a whole
//! multiple of the frame. The viewer watched is never touched: its battle, its sound, its kept states and its
//! position are its own, as the export's thread's are. Nothing of the engine is new; this is the library's seeking
//! (docs/frontend.md §8, "Seeking") on another instance.
//!
//! **What it costs.** A silent viewer plays a tick in about 17.5 µs (no sound is made), so indexing a five-minute
//! set takes about 0.3 s of the thread's time, once, and keeps about 77 states of the battle and the two HUDs
//! (some 2.5 MB, without the sound's 30 KB a state). A preview is then a seek of under 240 ticks (about 4 ms) and
//! one picture. They are asked one at a time, the latest hovered only, and no oftener than [`ASK_GAP`]; and kept by
//! the second they show ([`Bucket`]), up to [`CACHE_BYTES`] of them, so a second hovered again is shown at once. A
//! preview not drawn yet shows the nearest one kept meanwhile.

use crate::stage;
use nettai_frontend::game::Loaded;
use nettai_frontend::player::{tick_at, time_of};
use nettai_frontend::viewer::Viewer;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

/// The preview's size, in the frame's pixels: the frame itself (in the UI's units, as large; at the window's density
/// in the display's pixels).
pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;

/// The least time between two previews asked for (a few a second, as the pointer moves).
const ASK_GAP: Duration = Duration::from_millis(120);

/// The most the previews kept take, in bytes (the oldest seen go first): about 50 of them at twice the frame, 200
/// at the frame itself.
const CACHE_BYTES: usize = 32 << 20;

/// The ticks the thread indexes at a time while it has nothing asked of it (about 10 ms of its time).
const INDEX_SLICE: u32 = 600;

/// A preview's place in the cache: the replay's second it shows (each shows that second's first tick), the console,
/// and the multiple of the frame it was drawn at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bucket {
    pub second: u32,
    pub side: u8,
    pub density: u32,
}

impl Bucket {
    /// The bucket of `tick`: the second it is in.
    pub fn of(tick: u32, side: u8, density: u32) -> Bucket {
        Bucket { second: time_of(tick as u64).as_secs() as u32, side, density }
    }

    /// The tick a bucket's preview shows: its second's first.
    pub fn tick(&self) -> u32 {
        let mut tick = tick_at(Duration::from_secs(self.second as u64)) as u32;
        while tick > 0 && time_of(tick as u64 - 1).as_secs() >= self.second as u64 {
            tick -= 1;
        }
        while time_of(tick as u64).as_secs() < self.second as u64 {
            tick += 1;
        }
        tick
    }
}

/// The previews drawn, by bucket, the most lately used last; at most `most` bytes of them.
pub struct Cache<T> {
    entries: Vec<(Bucket, T, usize)>,
    bytes: usize,
    most: usize,
}

impl<T> Cache<T> {
    pub fn new(most: usize) -> Cache<T> {
        Cache { entries: Vec::new(), bytes: 0, most }
    }

    /// The preview of `bucket`, if it is kept (it is then the most lately used).
    pub fn get(&mut self, bucket: &Bucket) -> Option<&T> {
        let at = self.entries.iter().position(|(b, ..)| b == bucket)?;
        let entry = self.entries.remove(at);
        self.entries.push(entry);
        self.entries.last().map(|(_, t, _)| t)
    }

    /// Keep `bucket`'s preview, of `bytes`: the least lately used go while there are more than the most.
    pub fn put(&mut self, bucket: Bucket, value: T, bytes: usize) {
        if let Some(at) = self.entries.iter().position(|(b, ..)| *b == bucket) {
            self.bytes -= self.entries.remove(at).2;
        }
        self.entries.push((bucket, value, bytes));
        self.bytes += bytes;
        while self.bytes > self.most && self.entries.len() > 1 {
            self.bytes -= self.entries.remove(0).2;
        }
    }

    /// The kept preview nearest in time to `bucket`, of its console and density: what shows while its own is drawn.
    pub fn nearest(&self, bucket: &Bucket) -> Option<(Bucket, &T)> {
        self.entries
            .iter()
            .filter(|(b, ..)| b.side == bucket.side && b.density == bucket.density)
            .min_by_key(|(b, ..)| b.second.abs_diff(bucket.second))
            .map(|(b, t, _)| (*b, t))
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// A preview asked of the thread.
struct Ask {
    bucket: Bucket,
    tick: u32,
}

/// A preview drawn: the frame's pixels at its bucket's density, 0x00RRGGBB.
struct Drawn {
    bucket: Bucket,
    pixels: Vec<u32>,
}

/// The previews of one replay, while it is watched. Dropped, the thread is told (its channel closes) and ends at its
/// next look.
pub struct Thumbnails {
    asks: Sender<Ask>,
    drawn: Receiver<Drawn>,
    cache: Cache<slint::Image>,
    /// The bucket under the pointer; the one asked for and not back; when the last was asked for.
    hovered: Option<Bucket>,
    asked: Option<Bucket>,
    last_ask: Option<Instant>,
    /// The bucket whose preview the view has (none: it has none).
    shown: Option<Bucket>,
    /// The thread ended (the engine stopped it, or it couldn't open the replay): no more are asked for.
    ended: bool,
}

impl Thumbnails {
    /// Start the thread for the replay at `path`, played on `game` (whose sound is left out: none is made).
    pub fn start(game: Loaded, path: PathBuf) -> Thumbnails {
        let (asks, asked) = mpsc::channel();
        let (done, drawn) = mpsc::channel();
        let started = std::thread::Builder::new().name("previews".into()).spawn(move || work(game, path, asked, done));
        if let Err(why) = &started {
            crate::errors::log("starting the bar's previews", why);
        }
        Thumbnails { asks, drawn, cache: Cache::new(CACHE_BYTES), hovered: None, asked: None, last_ask: None, shown: None, ended: started.is_err() }
    }

    /// The pointer is over the bar at `bucket`'s time, or (none) left it.
    pub fn hover(&mut self, bucket: Option<Bucket>) {
        self.hovered = bucket;
    }

    /// A frame at `now`: the previews drawn since are kept, and the next is asked for where it is due. The preview to
    /// show, where it changed: the hovered one's, else the nearest kept meanwhile.
    pub fn frame(&mut self, now: Instant) -> Option<slint::Image> {
        loop {
            match self.drawn.try_recv() {
                Ok(Drawn { bucket, pixels }) => {
                    let (w, h) = (WIDTH * bucket.density as usize, HEIGHT * bucket.density as usize);
                    let bytes = pixels.len() * 4;
                    self.cache.put(bucket, stage::image_of(&pixels, w, h), bytes);
                    if self.asked == Some(bucket) {
                        self.asked = None;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.ended = true;
                    self.asked = None;
                    break;
                }
            }
        }
        let hovered = self.hovered?;
        if let Some(image) = self.cache.get(&hovered).cloned() {
            return (self.shown != Some(hovered)).then(|| {
                self.shown = Some(hovered);
                image
            });
        }
        let due = self.last_ask.is_none_or(|at| now.saturating_duration_since(at) >= ASK_GAP);
        if !self.ended && self.asked.is_none() && due {
            if self.asks.send(Ask { bucket: hovered, tick: hovered.tick() }).is_ok() {
                self.asked = Some(hovered);
                self.last_ask = Some(now);
            } else {
                self.ended = true;
            }
        }
        let (near, image) = self.cache.nearest(&hovered)?;
        (self.shown != Some(near)).then(|| {
            self.shown = Some(near);
            image.clone()
        })
    }

    /// Whether the view has a preview to show (one has been drawn of this console and density).
    pub fn has(&self, bucket: &Bucket) -> bool {
        self.cache.nearest(bucket).is_some()
    }
}

/// The thread: a silent viewer of the replay, indexing ahead while nothing is asked, and drawing the latest preview
/// asked for. It ends when the screen drops its end of the channel, or when the engine stops it.
fn work(game: Loaded, path: PathBuf, asks: Receiver<Ask>, drawn: Sender<Drawn>) {
    let doing = format!("previewing {}", path.display());
    let replay = match crate::replays::read(&path) {
        Ok(replay) => replay,
        Err(why) => return crate::errors::log(&doing, &why),
    };
    let mut viewer = match nettai_frontend::session::in_engine(|| Viewer::open(&game, &replay, None)) {
        Ok(Ok(viewer)) => viewer,
        Ok(Err(refused)) => return crate::errors::log(&doing, &refused),
        Err(panic) => return crate::errors::log(&doing, &format_args!("the engine stopped: {}", nettai_frontend::session::panic_message(panic.as_ref()))),
    };
    viewer.set_paused(true);
    let mut whole = false;
    loop {
        let ask = if whole {
            match asks.recv() {
                Ok(ask) => ask,
                Err(_) => return,
            }
        } else {
            match asks.try_recv() {
                Ok(ask) => ask,
                Err(TryRecvError::Empty) => {
                    match nettai_frontend::session::in_engine(|| viewer.index(INDEX_SLICE)) {
                        Ok(done) => whole = done,
                        Err(panic) => return crate::errors::log(&doing, &format_args!("the engine stopped indexing: {}", nettai_frontend::session::panic_message(panic.as_ref()))),
                    }
                    continue;
                }
                Err(TryRecvError::Disconnected) => return,
            }
        };
        // (The latest asked for: the ones before it were passed by.)
        let ask = asks.try_iter().last().unwrap_or(ask);
        let density = ask.bucket.density.max(1) as usize;
        let (w, h) = (WIDTH * density, HEIGHT * density);
        let mut pixels = vec![0u32; w * h];
        let drew = nettai_frontend::session::in_engine(|| {
            viewer.seek(ask.tick);
            viewer.present_view(ask.bucket.side, &mut pixels, w, h);
        });
        if let Err(panic) = drew {
            return crate::errors::log(&doing, &format_args!("the engine stopped at tick {}: {}", ask.tick, nettai_frontend::session::panic_message(panic.as_ref())));
        }
        if drawn.send(Drawn { bucket: ask.bucket, pixels }).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tick's bucket is the second it is in, and a bucket's tick is that second's first: each tick of a second
    /// keys one preview, and the preview shows a tick of its own second.
    #[test]
    fn a_preview_is_keyed_by_its_second() {
        for tick in [0u32, 1, 59, 60, 61, 119, 5973, 5974, 17_999, 37_187] {
            let bucket = Bucket::of(tick, 0, 2);
            assert_eq!(bucket.second as u64, time_of(tick as u64).as_secs(), "{tick}");
            let first = bucket.tick();
            assert!(first <= tick, "{tick}: its second begins at {first}");
            assert_eq!(Bucket::of(first, 0, 2), bucket, "{tick}");
            assert!(first == 0 || Bucket::of(first - 1, 0, 2).second < bucket.second, "{tick}: {first} is its second's first");
        }
        // (Each second has its bucket, in order.)
        let seconds: Vec<u32> = (0..600).map(|t| Bucket::of(t, 1, 1).second).collect();
        assert!(seconds.windows(2).all(|w| w[1] == w[0] || w[1] == w[0] + 1));
        assert_eq!(*seconds.last().unwrap(), 10);
    }

    /// The console and the density are part of the key: a preview of one console or size isn't another's.
    #[test]
    fn a_preview_is_its_consoles_and_its_sizes() {
        let mut cache = Cache::new(1 << 20);
        cache.put(Bucket::of(600, 0, 1), "P1 at 10 s", 10);
        assert_eq!(cache.get(&Bucket::of(620, 0, 1)), Some(&"P1 at 10 s"));
        assert_eq!(cache.get(&Bucket::of(600, 1, 1)), None);
        assert_eq!(cache.get(&Bucket::of(600, 0, 2)), None);
        assert_eq!(cache.get(&Bucket::of(660, 0, 1)), None);
        // (Meanwhile the nearest kept of the same console and density shows.)
        cache.put(Bucket::of(1800, 0, 1), "P1 at 30 s", 10);
        cache.put(Bucket::of(1200, 1, 1), "P2 at 20 s", 10);
        assert_eq!(cache.nearest(&Bucket::of(1500, 0, 1)).map(|(_, t)| *t), Some("P1 at 30 s"));
        assert_eq!(cache.nearest(&Bucket::of(900, 0, 1)).map(|(_, t)| *t), Some("P1 at 10 s"));
        assert_eq!(cache.nearest(&Bucket::of(900, 1, 1)).map(|(_, t)| *t), Some("P2 at 20 s"));
        assert_eq!(cache.nearest(&Bucket::of(900, 1, 2)), None);
    }

    /// The cache keeps at most its bytes, the least lately used going first; a preview kept again replaces itself.
    #[test]
    fn the_cache_keeps_the_latest_used() {
        let mut cache = Cache::new(30);
        let at = |second: u32| Bucket { second, side: 0, density: 1 };
        for second in 0..3 {
            cache.put(at(second), second, 10);
        }
        assert_eq!(cache.get(&at(0)), Some(&0));
        cache.put(at(3), 3, 10);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&at(1)), None, "the least lately used went");
        assert_eq!(cache.get(&at(0)), Some(&0));
        cache.put(at(3), 33, 10);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&at(3)), Some(&33));
    }
}
