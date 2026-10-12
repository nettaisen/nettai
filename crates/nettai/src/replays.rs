//! The replays folder, read off the window's thread (docs/app.md §15): what the Replays screen lists and the
//! main menu's reel plays.
//!
//! Nothing here runs on the frame path. A [`Library`] has a thread of its own, started when it is first asked
//! something, which lists the folder (each file read and its head, info and length kept: [`Entry`]), plays a
//! replay out for who won ([`Library::play`]: the library's `play_out`, with no picture or sound) and makes a
//! still of one ([`Library::still`]). The screen asks, and takes what was found at its next frame
//! ([`Library::poll`]). A file that doesn't read is listed with why ([`Entry::read`]), never a failure of the
//! listing; a replay that isn't played here is told as its outcome ([`Played`]).
//!
//! The main menu's reel has a smaller thing, [`Reel`]: the selected game's latest few replays, read on a thread
//! as the menu shows, then handed out one after another.

use nettai_battle::Content;
use nettai_battle::battle::mode;
use nettai_frontend::game::{Graphics, TextMode};
use nettai_frontend::replay::{Info, Outcome, ReadError, Refused, Replay};
use nettai_frontend::viewer::Viewer;
use nettai_render::Frame;
use nettai_render::vfont::VectorFont;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{SystemTime, UNIX_EPOCH};

/// The replays the reel goes through: the latest few.
const REEL: usize = 5;

/// The ticks of fighting a still is taken after: a moment into the first fight, not its custom screen.
const STILL_AFTER: u32 = 150;

/// The folder the replays are listed from: `crate::paths::replays`. (The screenshot pass has replays of its
/// own, in its folder's `replays`: its pictures don't depend on what the player has recorded.)
pub fn folder() -> PathBuf {
    match std::env::var_os("NETTAI_TOUR") {
        Some(tour) => PathBuf::from(tour).join("replays"),
        None => crate::paths::replays(),
    }
}

/// Which file an entry was read from: one written again since (a recording that went on) is another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

/// What a replay's file says of itself, without playing it.
#[derive(Clone, Debug)]
pub struct Summary {
    /// The engine, the game and the content it plays on.
    pub head: nettai_replay::Head,
    pub info: Info,
    /// Its length, in ticks.
    pub ticks: u32,
    /// Its rounds (each one's first tick).
    pub rounds: Vec<u32>,
}

impl Summary {
    fn of(replay: Replay) -> Summary {
        Summary { ticks: replay.ticks.len() as u32, rounds: replay.round_starts(), head: replay.head, info: replay.info }
    }
}

/// A file of the replays folder.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub stamp: Stamp,
    /// When it was played, in seconds since 1970: its info's, or for a file that doesn't read (or says no
    /// time) the time it was last written.
    pub when: u64,
    /// What it says, or why it doesn't read.
    pub read: Result<Summary, ReadError>,
}

impl Entry {
    fn key(&self) -> Key {
        (self.path.clone(), self.stamp)
    }
}

type Key = (PathBuf, Stamp);

/// What playing a replay out made: what happened in it, or why it isn't played here.
pub type Played = Result<Outcome, Refused>;

/// What a still is drawn with: the game's graphics in the language shown, and the text as the settings say.
#[derive(Clone)]
pub struct Look {
    pub graphics: Graphics,
    pub text: TextMode,
    pub font: Option<Arc<VectorFont>>,
}

enum Job {
    List(PathBuf),
    Play(Key, Arc<Content>),
    Still(Key, u8, Arc<Content>, Box<Look>),
}

enum Told {
    Listed(Vec<Entry>),
    Played(Key, Played),
    Still(Key, Option<Box<Frame>>),
}

/// What [`Library::poll`] says was found since the last.
pub enum Found {
    /// The folder was listed: [`Library::entries`] is new.
    Listed,
    /// A replay was played out: [`Library::played`] has it.
    Played(PathBuf),
    /// A replay's still (none: it isn't played here).
    Still(PathBuf, Option<Box<Frame>>),
}

/// The replays folder as last listed, and what was found of its replays.
#[derive(Default)]
pub struct Library {
    worker: Option<(Sender<Job>, Receiver<Told>)>,
    /// The files, newest first.
    pub entries: Vec<Entry>,
    /// The folder was listed at least once.
    pub listed: bool,
    played: HashMap<Key, Played>,
    /// What the thread was asked to play out and hasn't yet.
    asked: HashSet<Key>,
}

impl Library {
    fn ask(&mut self, job: Job) {
        let (jobs, _) = self.worker.get_or_insert_with(|| {
            let (jobs, taken) = channel();
            let (tell, told) = channel();
            std::thread::Builder::new().name("replays".into()).spawn(move || work(taken, tell)).expect("a thread can be started");
            (jobs, told)
        });
        let _ = jobs.send(job);
    }

    /// List `dir` again (a match just recorded is there): [`Found::Listed`] says when.
    pub fn list(&mut self, dir: &Path) {
        self.ask(Job::List(dir.to_path_buf()));
    }

    /// Play `entry` out on `content`, if that wasn't done or asked: [`Found::Played`] says when.
    pub fn play(&mut self, entry: &Entry, content: &Arc<Content>) {
        let key = entry.key();
        if self.played.contains_key(&key) || !self.asked.insert(key.clone()) {
            return;
        }
        self.ask(Job::Play(key, content.clone()));
    }

    /// What playing `entry` out made, once it was.
    pub fn played(&self, entry: &Entry) -> Option<&Played> {
        self.played.get(&(entry.path.clone(), entry.stamp))
    }

    /// Make a still of `entry` from `side`'s console: [`Found::Still`] brings it. One asked for after it is
    /// made first, and one asked for before it and not yet begun isn't made at all.
    pub fn still(&mut self, entry: &Entry, side: u8, content: &Arc<Content>, look: Look) {
        self.ask(Job::Still(entry.key(), side, content.clone(), Box::new(look)));
    }

    /// Take what the thread found since the last call.
    pub fn poll(&mut self) -> Vec<Found> {
        let mut found = Vec::new();
        let Some((_, told)) = &self.worker else { return found };
        while let Ok(told) = told.try_recv() {
            match told {
                Told::Listed(entries) => {
                    self.entries = entries;
                    self.listed = true;
                    // (What was found of files that are gone, or written again, is of no use.)
                    let there: HashSet<Key> = self.entries.iter().map(Entry::key).collect();
                    self.played.retain(|key, _| there.contains(key));
                    found.push(Found::Listed);
                }
                Told::Played(key, played) => {
                    self.asked.remove(&key);
                    found.push(Found::Played(key.0.clone()));
                    self.played.insert(key, played);
                }
                Told::Still(key, frame) => found.push(Found::Still(key.0, frame)),
            }
        }
        found
    }
}

/// The thread: a listing first, then the still asked for last, then the replays to play out, oldest asked
/// first.
fn work(jobs: Receiver<Job>, tell: Sender<Told>) {
    let mut listing: Option<PathBuf> = None;
    let mut still: Option<Job> = None;
    let mut plays: VecDeque<(Key, Arc<Content>)> = VecDeque::new();
    // (What the last listing read: a file as it was then is not read again.)
    let mut known: HashMap<PathBuf, Entry> = HashMap::new();
    loop {
        // What was asked meanwhile; with nothing to do, wait for the next.
        let idle = listing.is_none() && still.is_none() && plays.is_empty();
        let mut next = if idle { jobs.recv().ok() } else { jobs.try_recv().ok() };
        if idle && next.is_none() {
            return;
        }
        while let Some(job) = next {
            match job {
                Job::List(dir) => listing = Some(dir),
                Job::Play(key, content) => plays.push_back((key, content)),
                job @ Job::Still(..) => still = Some(job),
            }
            next = jobs.try_recv().ok();
        }
        let told = if let Some(dir) = listing.take() {
            let entries = list(&dir, &known);
            known = entries.iter().map(|e| (e.path.clone(), e.clone())).collect();
            Told::Listed(entries)
        } else if let Some(Job::Still(key, side, content, look)) = still.take() {
            let frame = guarded(|| still_of(&key.0, side, &content, &look)).flatten();
            Told::Still(key, frame.map(Box::new))
        } else if let Some((key, content)) = plays.pop_front() {
            let played = guarded(|| play(&key.0, &content)).unwrap_or_else(|| Err(Refused::BadMatch("playing it out stopped the engine".into())));
            Told::Played(key, played)
        } else {
            continue;
        };
        if tell.send(told).is_err() {
            return;
        }
    }
}

/// Run `f`; none if it panicked (the engine stopping on a replay's match is that replay's, not the thread's).
fn guarded<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

/// The replays in `dir`, newest first: each file read, but one that is as `known` has it.
fn list(dir: &Path, known: &HashMap<PathBuf, Entry>) -> Vec<Entry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Entry> = read.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "ntrp")).filter_map(|path| entry(path, known)).collect();
    out.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| b.path.cmp(&a.path)));
    out
}

fn entry(path: PathBuf, known: &HashMap<PathBuf, Entry>) -> Option<Entry> {
    let meta = std::fs::metadata(&path).ok()?;
    let stamp = Stamp { len: meta.len(), modified: meta.modified().ok() };
    if let Some(was) = known.get(&path).filter(|e| e.stamp == stamp) {
        return Some(was.clone());
    }
    let read = read(&path).map(Summary::of);
    let written = || stamp.modified.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    let when = read.as_ref().ok().map(|s| s.info.when).filter(|&w| w > 0).unwrap_or_else(written);
    Some(Entry { path, stamp, when, read })
}

/// A replay's file, read. (One that can't be read at all is a damaged one.)
pub fn read(path: &Path) -> Result<Replay, ReadError> {
    let bytes = std::fs::read(path).map_err(|e| ReadError::Damaged(format!("{}: {e}", path.display())))?;
    Replay::read(&bytes)
}

fn play(path: &Path, content: &Arc<Content>) -> Played {
    let replay = read(path).map_err(|e| Refused::BadMatch(e.to_string()))?;
    nettai_frontend::replay::play_out(content, &replay)
}

/// A picture of the replay at `path` from `side`'s console: a moment into its first fight (or its last tick,
/// if it has no such moment). None if it isn't played here.
fn still_of(path: &Path, side: u8, content: &Arc<Content>, look: &Look) -> Option<Frame> {
    let replay = read(path).ok()?;
    let renderer = look.graphics.renderer(look.text, look.font.clone());
    let mut viewer = Viewer::with(content, &replay, Some(side), renderer, None, None).ok()?;
    let mut fought = 0;
    while fought < STILL_AFTER && viewer.step_forward() {
        fought += (viewer.battle().round.mode == mode::FIGHTING) as u32;
    }
    Some(viewer.frame())
}

/// The main menu's reel's source: the player's latest replays of a game, one after another, around from the
/// last to the first. They are found and read on a thread as it is made.
pub struct Reel {
    found: Receiver<Vec<Replay>>,
    replays: Option<Vec<Replay>>,
    next: usize,
}

/// What the reel plays next.
pub enum Next {
    /// The replays are still being read: ask again.
    Wait,
    /// This replay.
    Replay(Box<Replay>),
    /// The player has none of the game that plays: something made for the reel.
    Nothing,
}

impl Reel {
    /// The latest replays of `game` in `dir`.
    pub fn of(game: &str, dir: PathBuf) -> Reel {
        let (tell, found) = channel();
        let game = game.to_string();
        std::thread::spawn(move || {
            let latest = list(&dir, &HashMap::new()).into_iter().filter(|e| e.read.as_ref().is_ok_and(|s| s.head.game == game)).take(REEL);
            // (One that doesn't read any more is passed over.)
            let _ = tell.send(latest.filter_map(|e| read(&e.path).ok()).collect());
        });
        Reel { found, replays: None, next: 0 }
    }

    /// A reel with no replays (the screenshot pass's).
    pub fn empty() -> Reel {
        Reel { found: channel().1, replays: Some(Vec::new()), next: 0 }
    }

    /// The next replay to play.
    pub fn next(&mut self) -> Next {
        if self.replays.is_none() {
            match self.found.try_recv() {
                Ok(replays) => self.replays = Some(replays),
                Err(std::sync::mpsc::TryRecvError::Empty) => return Next::Wait,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.replays = Some(Vec::new()),
            }
        }
        let replays = self.replays.as_ref().expect("read above");
        if replays.is_empty() {
            return Next::Nothing;
        }
        let replay = replays[self.next % replays.len()].clone();
        self.next += 1;
        Next::Replay(Box::new(replay))
    }

    /// The replay last given doesn't play here: it isn't given again.
    pub fn skip(&mut self) {
        if let Some(replays) = &mut self.replays
            && !replays.is_empty()
        {
            let at = (self.next - 1) % replays.len();
            replays.remove(at);
            // (The one after it is next.)
            self.next = at;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_frontend::replay::{End, MatchKind};

    /// A replay that says nothing but when it was played.
    fn replay(when: u64) -> Replay {
        let head = nettai_replay::Head { engine: String::new(), game: "a-game".into(), content: 0, start: 0 };
        Replay { head, match_bytes: Vec::new(), info: Info { when, ..Info::of(MatchKind::Online, 0) }, ticks: Vec::new(), end: End::Open }
    }

    fn given(reel: &mut Reel) -> Option<u64> {
        match reel.next() {
            Next::Replay(replay) => Some(replay.info.when),
            Next::Nothing => None,
            Next::Wait => panic!("the reel has its replays"),
        }
    }

    /// The reel gives its replays one after another, around from the last to the first; one that didn't play
    /// isn't given again, and the one after it is next; with none left it has nothing.
    #[test]
    fn the_reel_goes_around_and_passes_over_what_doesnt_play() {
        let mut reel = Reel { found: channel().1, replays: Some(vec![replay(3), replay(2), replay(1)]), next: 0 };
        assert_eq!([given(&mut reel), given(&mut reel), given(&mut reel), given(&mut reel)], [Some(3), Some(2), Some(1), Some(3)]);
        assert_eq!(given(&mut reel), Some(2));
        reel.skip();
        assert_eq!([given(&mut reel), given(&mut reel), given(&mut reel)], [Some(1), Some(3), Some(1)]);
        reel.skip();
        assert_eq!(given(&mut reel), Some(3));
        reel.skip();
        assert_eq!(given(&mut reel), None);
        assert_eq!(given(&mut Reel::empty()), None);
    }

    /// Until the thread has read the replays the reel says to wait; a thread that went without saying is a
    /// reel with none.
    #[test]
    fn the_reel_waits_for_its_replays() {
        let (tell, found) = channel();
        let mut reel = Reel { found, replays: None, next: 0 };
        assert!(matches!(reel.next(), Next::Wait));
        tell.send(vec![replay(7)]).unwrap();
        assert_eq!(given(&mut reel), Some(7));
        let mut gone = Reel { found: channel().1, replays: None, next: 0 };
        assert!(matches!(gone.next(), Next::Nothing));
    }
}
