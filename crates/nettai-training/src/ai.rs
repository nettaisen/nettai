//! The learned AI in live play (docs/design/training-ai.md §10): [`NetAi`],
//! live play's `driver::Ai`, plays a side with the trained network
//! ([`crate::net`]) as the env plays a policy seat in training (§4, §6), and
//! [`Fingerprinter`] makes its game's chips' fingerprints for it on threads
//! of their own.
//!
//! - **What it sees and when.** Every tick it is called on (the battle's
//!   thread: once a tick, `fight` or `custom_screen`), it counts the tick,
//!   keeps the attack log, and encodes the view into its reaction-delay ring
//!   on the ticks a decision will need (`(t + D) mod k = 0`). In the fight it
//!   decides every `k` ticks from the view of `D` ticks before (`D` its
//!   level's delay); on the custom screen it decides whenever its navigator
//!   is idle and the screen is choosing chips, from the screen as it is, the
//!   keys probed in a copy of the battle at its first decision there. It
//!   holds a fight action till the next decision; the navigator carries a
//!   screen choice out, a slot or an entry of the form list (act.rs). It never reads `theirs`: it doesn't hear the
//!   player's buttons.
//! - **Its memory.** The GRU state and the last decision's action (the
//!   network's next decision reads it: [`net::Memory::took`]) carry through
//!   a round and start over at the next (a new round is a new score of the
//!   set), and when the set starts over (`reset`), its draws too.
//! - **Its chips' fingerprints.** The chips it may see, as they are needed
//!   (a chip's fingerprint is its own alone, so made apart it is what the
//!   trainer's pool makes of it): both folders' chips and the Program
//!   Advances they make, asked for as each round starts (and by the host as
//!   it sets a session up: [`setup_chips`]). A [`Fingerprinter`] reads them
//!   from the host's cache folder by content hash, or makes them on threads
//!   of its own and writes them there. A decision whose view shows a chip
//!   without one waits for it (in the fight the action held goes on, its
//!   custom screen waits); the network never sees a chip whose fingerprint
//!   isn't made, and the battle's thread never waits for one.
//! - **Its weights.** The host's: a network it has read ([`NetAi::of`],
//!   every `NetAi` of one sharing it), or one still arriving ([`Arriving`]:
//!   the app downloads its from the nettai-ai repository, or reads the file
//!   `NETTAI_AI_WEIGHTS` names), which [`WaitingAi`] waits for. None is
//!   built in.
//!
//! Nothing here names a game.

use crate::act::{self, Navigator};
use crate::danger;
use crate::env::{self, fight_runs};
use crate::fingerprint::{self, Fingerprints};
use crate::net::{self, Action, Heads, Knobs, Memory, Net};
use crate::observe::{self, AttackHistory, Extra, Names, Obs};
use crate::search::{Search, Worker};
use crate::spec;
use nettai_battle::battle::mode;
use nettai_battle::input::keys;
use nettai_battle::setup::SetScore;
use nettai_battle::{Battle, Content};
use nettai_frontend::driver::Ai;
use nettai_netplay::rng::SplitMix64;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// A network a host plays with that may still be on its way (the app's is
/// downloaded: docs/design/training-ai.md §10.5): it arrives once, from
/// whichever thread has it ([`Arriving::arrive`]), and every AI waiting on it
/// shares it ([`WaitingAi`]). Err: why there is none (it couldn't be had, or
/// this encoder can't play it).
#[derive(Clone, Default)]
pub struct Arriving(Arc<OnceLock<Result<Arc<Net>, String>>>);

impl Arriving {
    /// One that has arrived already.
    pub fn arrived(net: Result<Arc<Net>, String>) -> Arriving {
        let arriving = Arriving::default();
        arriving.arrive(net);
        arriving
    }

    /// It arrives (once: what comes after is dropped).
    pub fn arrive(&self, net: Result<Arc<Net>, String>) {
        let _ = self.0.set(net);
    }

    /// The network, once it has arrived (or why there is none).
    pub fn get(&self) -> Option<&Result<Arc<Net>, String>> {
        self.0.get()
    }
}

/// How well it plays: one of the levels the weights carry knobs for (their
/// `levels` metadata: the reaction delay, how it chooses).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    Easy,
    Normal,
    Hard,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Easy, Level::Normal, Level::Hard];

    /// How it looks ahead at this level (§10.7): Hard does, Easy and
    /// Normal play the network's choice.
    pub fn search(self) -> Option<Search> {
        (self == Level::Hard).then(Search::default)
    }

    /// Its name in the weights' metadata.
    pub fn name(self) -> &'static str {
        match self {
            Level::Easy => "easy",
            Level::Normal => "normal",
            Level::Hard => "hard",
        }
    }
}

/// Where a game's fingerprints are, of the chips asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The cache folder being read.
    Reading,
    /// Being made: chips done of those asked for (those read included).
    Making { done: usize, of: usize },
    /// Every chip asked for is made.
    Ready,
    Failed(String),
}

/// The fingerprints of a content's chips for live play, made as they are
/// asked for ([`Fingerprinter::want`]: the two folders' chips as a session
/// starts, and any other the AI comes to see), on threads of their own
/// (`nettai_platform::thread::spawn`: the browser's workers too), and kept in
/// a cache folder by content hash (`fingerprints-<game>-<hash>.json`, as the
/// trainer's pool keeps a whole game's), so a chip is made once. A chip's
/// fingerprint is its own alone, so made apart it is what the pool makes.
/// One a content in a process: another [`Fingerprinter::start`] of the same
/// content is the same one.
#[derive(Clone)]
pub struct Fingerprinter(Arc<Shared>);

struct Shared {
    /// The content (none: fingerprints a host gave, made by no thread).
    content: Option<Arc<Content>>,
    hash: u64,
    /// The chips that have fingerprints (the game's own).
    printable: std::collections::HashSet<nettai_content_api::ChipHandle>,
    file: Option<PathBuf>,
    state: Mutex<Made>,
    /// Workers wait on it for chips asked for.
    asked: std::sync::Condvar,
    /// The fingerprints made so far, given out whole, and how many times
    /// they have been (a reader that has seen this many has them all).
    given: Mutex<Arc<Fingerprints>>,
    generation: std::sync::atomic::AtomicU64,
}

#[derive(Default)]
struct Made {
    /// The cache folder has been read.
    read: bool,
    queue: std::collections::VecDeque<nettai_content_api::ChipHandle>,
    wanted: std::collections::HashSet<nettai_content_api::ChipHandle>,
    made: HashMap<nettai_content_api::ChipHandle, fingerprint::Fingerprint>,
    busy: usize,
    error: Option<String>,
    /// Why the fingerprints made couldn't be written to the cache folder.
    unkept: Option<String>,
}

impl Fingerprinter {
    /// The fingerprints of `content`'s chips, made as they are asked for on
    /// `threads` threads (at least one), kept in `cache`. The threads start at once (the
    /// first reads the cache folder); this returns at once. A thread that
    /// can't be started fails it.
    pub fn start(content: &Arc<Content>, threads: usize, cache: Option<PathBuf>) -> Fingerprinter {
        static STARTED: OnceLock<Mutex<HashMap<u64, Fingerprinter>>> = OnceLock::new();
        let hash = content.hash().0;
        let started = STARTED.get_or_init(Default::default);
        let mut started = nettai_platform::thread::lock(started).unwrap_or_else(|p| p.into_inner());
        if let Some(f) = started.get(&hash) {
            return f.clone();
        }
        let f = Fingerprinter::begin(content, threads.max(1), cache);
        started.insert(hash, f.clone());
        f
    }

    /// A fingerprinter of its own (not the process's one of the content).
    fn begin(content: &Arc<Content>, threads: usize, cache: Option<PathBuf>) -> Fingerprinter {
        let hash = content.hash().0;
        let empty = Fingerprints { version: fingerprint::VERSION, hash, ..Fingerprints::default() };
        let f = Fingerprinter(Arc::new(Shared {
            content: Some(content.clone()),
            hash,
            printable: fingerprint::game_chips(content).into_iter().collect(),
            file: cache.map(|d| d.join(fingerprint::file_name(content))),
            state: Mutex::new(Made::default()),
            asked: std::sync::Condvar::new(),
            given: Mutex::new(Arc::new(empty)),
            generation: Default::default(),
        }));
        if threads == 0 {
            // (No thread: none is read either.)
            nettai_platform::thread::lock(&f.0.state).unwrap_or_else(|p| p.into_inner()).read = true;
        }
        for k in 0..threads {
            let shared = f.0.clone();
            if let Err(e) = nettai_platform::thread::spawn(&format!("fingerprints {k}"), move || work(&shared, k == 0)) {
                f.0.fail(format!("a thread to make the chips' fingerprints on: {e}"));
                break;
            }
        }
        f
    }

    /// A fingerprinter with no thread (tests: chips asked for and never
    /// made).
    #[cfg(test)]
    fn idle(content: &Arc<Content>) -> Fingerprinter {
        Fingerprinter::begin(content, 0, None)
    }

    /// Fingerprints a host has already, all it will ask for: no thread.
    pub fn made(fingerprints: Arc<Fingerprints>) -> Fingerprinter {
        let content_free = Shared {
            content: None,
            hash: fingerprints.hash,
            printable: fingerprints.chips.keys().copied().collect(),
            file: None,
            state: Mutex::new(Made { read: true, ..Made::default() }),
            asked: std::sync::Condvar::new(),
            given: Mutex::new(fingerprints),
            generation: std::sync::atomic::AtomicU64::new(1),
        };
        Fingerprinter(Arc::new(content_free))
    }

    /// Make the fingerprints of `chips` (those of the game's own not made or
    /// asked for already), the threads told.
    pub fn want(&self, chips: impl IntoIterator<Item = nettai_content_api::ChipHandle>) {
        let s = &self.0;
        let mut m = nettai_platform::thread::lock(&s.state).unwrap_or_else(|p| p.into_inner());
        let mut more = false;
        for c in chips {
            if s.printable.contains(&c) && m.wanted.insert(c) {
                m.queue.push_back(c);
                more = true;
            }
        }
        drop(m);
        if more {
            s.asked.notify_all();
        }
    }

    /// Whether `chip` has a fingerprint once made (one of the game's own): a
    /// view that describes it without one waits for it.
    pub fn printable(&self, chip: nettai_content_api::ChipHandle) -> bool {
        self.0.printable.contains(&chip)
    }

    /// The fingerprints made so far, and how many times they were given out
    /// (another count: more of them).
    pub fn given(&self) -> (u64, Arc<Fingerprints>) {
        let generation = self.0.generation.load(std::sync::atomic::Ordering::Acquire);
        (generation, nettai_platform::thread::lock(&self.0.given).unwrap_or_else(|p| p.into_inner()).clone())
    }

    /// How many times they were given out (cheap: no lock).
    pub fn generation(&self) -> u64 {
        self.0.generation.load(std::sync::atomic::Ordering::Acquire)
    }

    /// The content hash they are of.
    pub fn hash(&self) -> u64 {
        self.0.hash
    }

    pub fn status(&self) -> Status {
        let m = nettai_platform::thread::lock(&self.0.state).unwrap_or_else(|p| p.into_inner());
        match (&m.error, m.read) {
            (Some(e), _) => Status::Failed(e.clone()),
            (None, false) => Status::Reading,
            (None, true) => {
                let of = m.wanted.len();
                let done = m.wanted.iter().filter(|c| m.made.contains_key(c)).count();
                if done == of { Status::Ready } else { Status::Making { done, of } }
            }
        }
    }

    /// Why it failed, if it did.
    pub fn error(&self) -> Option<String> {
        nettai_platform::thread::lock(&self.0.state).unwrap_or_else(|p| p.into_inner()).error.clone()
    }

    /// Why the fingerprints made couldn't be written to the cache folder
    /// (they are made again next time), if they couldn't.
    pub fn unkept(&self) -> Option<String> {
        nettai_platform::thread::lock(&self.0.state).unwrap_or_else(|p| p.into_inner()).unkept.clone()
    }
}

impl Shared {
    fn fail(&self, why: String) {
        // (The host's thread too, a thread that couldn't start: a page's main thread may not wait.)
        let mut m = nettai_platform::thread::lock(&self.state).unwrap_or_else(|p| p.into_inner());
        m.error.get_or_insert(why);
        drop(m);
        self.asked.notify_all();
    }

    /// The fingerprints made so far, given out.
    fn give(&self, m: &Made) {
        let f = Fingerprints { version: fingerprint::VERSION, hash: self.hash, chips: m.made.clone(), ..Fingerprints::default() };
        *self.given.lock().unwrap_or_else(|p| p.into_inner()) = Arc::new(f);
        self.generation.fetch_add(1, std::sync::atomic::Ordering::Release);
    }

    /// Every chip made written to the cache file (with what it held).
    fn keep(&self, m: &mut Made) {
        let Some(file) = &self.file else { return };
        let f = Fingerprints { version: fingerprint::VERSION, hash: self.hash, chips: m.made.clone(), ..Fingerprints::default() };
        let kept = serde_json::to_vec(&f).map_err(|e| e.to_string()).and_then(|bytes| {
            if let Some(dir) = file.parent() {
                nettai_platform::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            nettai_platform::fs::write(file, bytes).map_err(|e| format!("{}: {e}", file.display()))
        });
        m.unkept = kept.err();
    }
}

/// A fingerprinting thread: (the first) the cache folder read, then the
/// content's runtime and a sandbox loaded, then each chip asked for as it
/// is; when none is left to make, what was made is written to the cache.
fn work(s: &Shared, first: bool) {
    let Some(content) = s.content.clone() else { return };
    if first {
        let read = s.file.as_ref().and_then(|f| nettai_platform::fs::read(f).ok()).and_then(|bytes| serde_json::from_slice::<Fingerprints>(&bytes).ok());
        let mut m = s.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(f) = read.filter(|f| f.version == fingerprint::VERSION && f.hash == s.hash) {
            m.made = f.chips;
        }
        m.read = true;
        s.give(&m);
        drop(m);
        s.asked.notify_all();
    }
    let loaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        nettai_battle::behavior::Behaviors::for_content(&content).map_err(|e| e.to_string())?;
        fingerprint::Sandbox::new(&content)
    }));
    let sandbox = match loaded {
        Ok(Ok(sandbox)) => sandbox,
        Ok(Err(e)) => return s.fail(e),
        Err(_) => return s.fail(format!("{}: the fingerprints' sandbox panicked as it loaded", content.game())),
    };
    let conditions = fingerprint::Conditions::of(&content);
    loop {
        let mut m = s.state.lock().unwrap_or_else(|p| p.into_inner());
        let chip = loop {
            if m.error.is_some() {
                return;
            }
            if m.read
                && let Some(c) = m.queue.pop_front()
            {
                if m.made.contains_key(&c) {
                    continue;
                }
                break c;
            }
            m = s.asked.wait(m).unwrap_or_else(|p| p.into_inner());
        };
        m.busy += 1;
        drop(m);
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sandbox.fingerprint(chip, &conditions)));
        let stops = sandbox.take_stops();
        let mut m = s.state.lock().unwrap_or_else(|p| p.into_inner());
        m.busy -= 1;
        match made {
            Ok((fp, _, _)) if stops.is_empty() => {
                m.made.insert(chip, fp);
            }
            Ok(_) => {
                drop(m);
                return s.fail(format!("a fingerprint run stopped: {}", stops.join("; ")));
            }
            Err(_) => {
                drop(m);
                return s.fail(format!("{}: a fingerprint run of chip {} panicked", content.game(), content.defs.chip(chip).key));
            }
        }
        s.give(&m);
        if m.queue.is_empty() && m.busy == 0 {
            s.keep(&mut m);
        }
    }
}

/// A fight decision the lookahead is thinking over: the tick it takes
/// effect at, its number on the lookahead's thread, and the tick of the
/// view it was made from.
struct Pending {
    at: u32,
    n: u64,
    #[cfg_attr(not(test), allow(dead_code))]
    seen: u32,
}

/// What it does on the screen and in the fight, from the round's start.
struct Round {
    /// The round's content and the set's score as it started: another is
    /// another round.
    key: (u64, SetScore),
    /// Ticks since the round's start (calls), as the env counts them.
    t: u32,
    log: AttackHistory,
    /// The views a fight decision may need, by `(s + D) / k` modulo its
    /// length: their ticks and them.
    ring: Vec<(Option<u32>, Obs)>,
    action: i32,
    nav: Navigator,
    probe: Option<[bool; act::KEYS.len()]>,
    decisions: u16,
    in_screen: bool,
    /// The tick it opens the custom screen from, once the gauge is full (the
    /// env's opener: `env::opener_wait`); none until it is.
    opener: Option<u32>,
}

/// Why a [`NetAi`] stopped, for its host to read after giving it away
/// ([`NetAi::stopped`]): what [`NetAi::error`] says.
#[derive(Clone)]
pub struct Stopped {
    why: Arc<Mutex<Option<String>>>,
    fingerprints: Fingerprinter,
}

impl Stopped {
    pub fn why(&self) -> Option<String> {
        let why = nettai_platform::thread::lock(&self.why).unwrap_or_else(|p| p.into_inner()).clone();
        why.or_else(|| self.fingerprints.error())
    }
}

/// The learned AI for live play (§10): a side played by the trained
/// network at a level, as the env plays a policy seat.
pub struct NetAi {
    net: Arc<Net>,
    knobs: Knobs,
    seed: u64,
    rng: SplitMix64,
    /// Ticks a fight action is held, and decisions on one custom screen
    /// before only OK is allowed: the env's in training, as the weights say.
    k: u32,
    screen_decisions: u16,
    fingerprints: Fingerprinter,
    /// The game's names with the fingerprints made so far, and how many
    /// times the fingerprinter had given them out then.
    names: Option<(Names, u64)>,
    mem: Memory,
    round: Option<Round>,
    obs: Obs,
    /// Why it stopped, if it did, and the same where its host reads it
    /// ([`NetAi::stopped`]).
    error: Option<String>,
    told: Arc<Mutex<Option<String>>>,
    /// Looking ahead at each fight decision ([`crate::search`]): none, the
    /// level's own choice from the network's. Its thread, the decision it
    /// is thinking over, and the network's memory after that decision.
    search: Option<Search>,
    worker: Option<Worker>,
    pending: Option<Pending>,
    thought: Memory,
    /// Each fight decision's tick and the tick of the view it was made from.
    #[cfg(test)]
    decided: Vec<(u32, u32)>,
}

impl NetAi {
    /// The AI of the network in `weights` (a weights file of the trainer's
    /// export) at `level`, its draws from `seed`, the game's chips'
    /// fingerprints from `fingerprints`. Refused: weights this encoder can't
    /// play, or without the level.
    pub fn new(weights: &[u8], level: Level, seed: u64, fingerprints: Fingerprinter) -> Result<NetAi, String> {
        NetAi::of(Arc::new(Net::from_safetensors(weights)?), level, seed, fingerprints)
    }

    /// The same, of a network already read.
    pub fn of(net: Arc<Net>, level: Level, seed: u64, fingerprints: Fingerprinter) -> Result<NetAi, String> {
        let knobs = net.level(level.name()).cloned().ok_or_else(|| format!("the weights have no level {}", level.name()))?;
        let (k, screen_decisions) = (net.k(), net.screen_decisions());
        NetAi {
            mem: net.memory(),
            thought: net.memory(),
            net,
            knobs,
            seed,
            rng: SplitMix64::new(seed),
            k,
            screen_decisions,
            fingerprints,
            names: None,
            round: None,
            obs: Obs::new(),
            error: None,
            told: Default::default(),
            search: None,
            worker: None,
            pending: None,
            #[cfg(test)]
            decided: Vec::new(),
        }
        .with_search(level.search())
    }

    /// The same, each fight decision looking ahead as `search` says (the
    /// level's delay kept; its choice from the network's replaced), on a
    /// thread of its own; none, the level's own choice. An error if the
    /// thread can't start.
    pub fn with_search(mut self, search: Option<Search>) -> Result<NetAi, String> {
        self.worker = match search {
            Some(_) => Some(self.worker.take().map_or_else(Worker::start, Ok)?),
            None => None,
        };
        self.search = search;
        self.pending = None;
        Ok(self)
    }

    /// Whether every chip asked for has its fingerprint (a decision whose
    /// view describes a chip without one waits for it).
    pub fn ready(&self) -> bool {
        self.fingerprints.status() == Status::Ready
    }

    /// Its fingerprints' status, for a host to show.
    pub fn fingerprints(&self) -> &Fingerprinter {
        &self.fingerprints
    }

    /// Why it stopped playing, if it did (its fingerprints failed or are of
    /// another content, a decision with nothing allowed): it presses nothing
    /// from then on.
    pub fn error(&self) -> Option<String> {
        self.error.clone().or_else(|| self.fingerprints.error())
    }

    /// What its host reads of it once it has given it away (live play's
    /// `LivePlayer::set_ai` takes it whole): why it stopped, if it did.
    pub fn stopped(&self) -> Stopped {
        Stopped { why: self.told.clone(), fingerprints: self.fingerprints.clone() }
    }

    /// It stops playing, for `why`.
    fn stop(&mut self, why: String) {
        *nettai_platform::thread::lock(&self.told).unwrap_or_else(|p| p.into_inner()) = Some(why.clone());
        self.error = Some(why);
    }

    /// The level's knobs it plays at.
    pub fn knobs(&self) -> &Knobs {
        &self.knobs
    }

    /// What its view takes besides the battle (no danger map: the fight's
    /// views add it, `danger::foresee`).
    fn extra(&self, r: &Round) -> Extra {
        Extra {
            delay: self.knobs.delay,
            log: r.log,
            danger: Default::default(),
            probe: r.probe,
            refused: r.nav.refused,
            screen_decisions: r.decisions,
            only_ok: r.decisions >= self.screen_decisions,
        }
    }

    /// This tick's buttons for side `side` of `b` (both of the trait's
    /// calls: the tick is one tick of the round either way).
    fn tick(&mut self, b: &Battle, side: u8) -> u16 {
        let key = (b.setup.content.0, b.setup.score);
        if self.round.as_ref().is_none_or(|r| r.key != key) {
            // (A new round: the GRU's state, the ring and the screen's from nothing.)
            let len = (self.knobs.delay as u32 / self.k) as usize + 2;
            self.round = Some(Round {
                key,
                t: 0,
                log: AttackHistory::default(),
                ring: (0..len).map(|_| (None, Obs::new())).collect(),
                action: 0,
                nav: Navigator::default(),
                probe: None,
                decisions: 0,
                in_screen: false,
                opener: None,
            });
            self.mem.reset();
            self.pending = None;
            if let Some(w) = &self.worker {
                w.load(b.content.clone());
            }
            // (Both folders' chips and the Program Advances they make: the
            // chips it may see, made as the round starts. The other side's
            // folder is read for this alone; only a chip of it shown used
            // reaches the view.)
            self.fingerprints.want(folders_chips(b));
        }
        let mut r = self.round.take().expect("a round");
        let held = self.play(b, side, &mut r);
        r.t += 1;
        self.round = Some(r);
        held
    }

    fn play(&mut self, b: &Battle, side: u8, r: &mut Round) -> u16 {
        let (k, d) = (self.k, self.knobs.delay as u32);
        // The tick the battle just ran: its attacks into the log, as the env
        // keeps it after each tick.
        if r.t > 0 {
            r.log.push(b);
        }
        if self.error.is_some() || self.fingerprints.error().is_some() {
            return 0;
        }
        if self.fingerprints.hash() != b.setup.content.0 {
            self.stop(format!("the fingerprints are of content {:016x}, the battle's is {}", self.fingerprints.hash(), b.setup.content));
            return 0;
        }
        // (The names given the fingerprints made since they last were.)
        let generation = self.fingerprints.generation();
        if self.names.as_ref().is_none_or(|(_, g)| *g != generation) {
            let (g, fps) = self.fingerprints.given();
            match &mut self.names {
                Some((names, at)) => {
                    names.set_fingerprints(fps);
                    *at = g;
                }
                None => self.names = Some((Names::with_fingerprints(&b.content, fps), g)),
            }
        }
        let decides = fight_runs(b) && b.player(side).is_some();
        // (Looking ahead, the fight decision due now, asked for `lead` ticks
        // ago: taken if it is still due, its answer waited for if it isn't
        // ready.)
        if let Some(p) = self.pending.take_if(|p| p.at <= r.t)
            && p.at == r.t
            && decides
        {
            match self.worker.as_ref().expect("the lookahead's thread").wait(p.n) {
                Ok(Some(a)) => {
                    #[cfg(test)]
                    self.decided.push((p.at, p.seen));
                    std::mem::swap(&mut self.mem, &mut self.thought);
                    r.action = a as i32;
                    self.mem.took(Action::Fight(a));
                }
                Ok(None) => self.stop("a fight decision with nothing allowed".into()),
                Err(why) => self.stop(why),
            }
            if self.error.is_some() {
                return 0;
            }
        }
        let names = &self.names.as_ref().expect("names").0;
        // The view a later decision will need (the env's ring).
        if r.t > 0 && (r.t + d).is_multiple_of(k) {
            let danger = danger::foresee(b, self.rng.next_u64());
            let extra = Extra { danger, ..self.extra(r) };
            let at = (((r.t + d) / k) as usize) % r.ring.len();
            let slot = &mut r.ring[at];
            slot.0 = Some(r.t);
            observe::encode(b, side, names, &extra, &mut slot.1);
        }
        // A decision (the env's `asks`, then `ask`).
        let custom = b.round.mode == mode::CUSTOM;
        let screen = b.custom.sides[side as usize].screen;
        if let Some(search) = self.search {
            // (Looking ahead: the decision `lead` ticks from now asked for,
            // from the view it will have, the network's memory after it
            // kept apart till it takes effect.)
            let lead = search.lead.clamp(1, k);
            let t = r.t + lead;
            if decides && t.is_multiple_of(k) {
                let want = t.checked_sub(d).filter(|&w| w <= r.t);
                let slot = &r.ring[((t / k) as usize) % r.ring.len()];
                let delayed = want.is_some() && slot.0 == want;
                let obs = if delayed {
                    &slot.1
                } else {
                    let danger = danger::foresee(b, self.rng.next_u64());
                    let extra = Extra { danger, ..self.extra(r) };
                    observe::encode(b, side, names, &extra, &mut self.obs);
                    &self.obs
                };
                if !waits(&self.fingerprints, obs) {
                    self.thought.h.copy_from_slice(&self.mem.h);
                    self.thought.last = self.mem.last;
                    let out = self.net.forward(obs, &mut self.thought, Heads::Fight);
                    let logits = out.fight.expect("the fight's head");
                    let luck = self.rng.next_u64();
                    let worker = self.worker.as_mut().expect("the lookahead's thread");
                    let n = worker.ask(search, b, side, k, act::fight_buttons(r.action), logits, luck);
                    self.pending = Some(Pending { at: t, n, seen: if delayed { t - d } else { r.t } });
                }
            }
        } else if decides && r.t.is_multiple_of(k) {
            let want = r.t.checked_sub(d);
            let slot = &r.ring[((r.t / k) as usize) % r.ring.len()];
            let delayed = want.is_some() && slot.0 == want;
            let obs = if delayed {
                &slot.1
            } else {
                // (A decision earlier than the delay: what it sees now.)
                let danger = danger::foresee(b, self.rng.next_u64());
                let extra = Extra { danger, ..self.extra(r) };
                observe::encode(b, side, names, &extra, &mut self.obs);
                &self.obs
            };
            if waits(&self.fingerprints, obs) {
                // (No decision on a view without a chip's fingerprint: the
                // action held.)
            } else {
                #[cfg(test)]
                self.decided.push((r.t, if delayed { r.t - d } else { r.t }));
                let out = self.net.forward(obs, &mut self.mem, Heads::Fight);
                let logits = out.fight.expect("the fight's head");
                match net::choose(&logits, &[true; spec::FIGHT_ACTIONS], &self.knobs, &mut self.rng) {
                    Some(a) => {
                        r.action = a as i32;
                        self.mem.took(Action::Fight(a));
                    }
                    None => self.stop("a fight decision with nothing allowed".into()),
                }
            }
        }
        // (The custom screen is open only while the fight doesn't run.)
        if custom && r.nav.idle() && screen.is_some_and(|s| act::chooses(&s)) {
            if r.probe.is_none() {
                r.probe = Some(env::probe(b, side, &mut r.nav));
            }
            let extra = self.extra(r);
            let names = &self.names.as_ref().expect("names").0;
            observe::encode(b, side, names, &extra, &mut self.obs);
            // (No decision on a view without a chip's fingerprint: the screen
            // waits, and asks again next tick.)
            if !waits(&self.fingerprints, &self.obs) {
                let legal = act::legal_screen(b, side, &r.nav, extra.only_ok);
                let out = self.net.forward(&self.obs, &mut self.mem, Heads::Screen);
                let logits = out.screen.expect("the screen's head");
                match net::choose(&logits, &legal, &self.knobs, &mut self.rng) {
                    Some(a) => {
                        r.nav.point(a as i32);
                        r.decisions += 1;
                        self.mem.took(Action::Screen(a));
                    }
                    None => self.stop("a custom screen decision with nothing allowed".into()),
                }
            }
        }
        // This tick's buttons (the env's `tick`): a new screen's navigator
        // from nothing and the fight action let go; on the screen the
        // navigator's, in the fight the action held.
        if custom && !r.in_screen {
            r.action = 0;
            r.nav.reset();
            r.probe = None;
            r.decisions = 0;
            r.opener = None;
        }
        r.in_screen = custom;
        if self.error.is_some() {
            0
        } else if custom {
            r.nav.buttons(b, side)
        } else if fight_runs(b) {
            // (It opens the custom screen as the env did in training, a while after the gauge is full: the user,
            // "allow the dummy to go into custom screen". The player may open it first.)
            let at = if env::may_open(b) { *r.opener.get_or_insert_with(|| r.t + env::opener_wait(self.rng.next_u64())) } else { u32::MAX };
            act::fight_buttons(r.action) | if env::presses_l(r.t, at) { keys::L } else { 0 }
        } else {
            0
        }
    }
}

/// Whether a decision on `obs` waits: it describes a chip of the game's
/// whose fingerprint wasn't made when it was encoded (the chip asked for).
fn waits(fingerprints: &Fingerprinter, obs: &Obs) -> bool {
    let missing: Vec<nettai_content_api::ChipHandle> = obs.unprinted.iter().copied().filter(|&c| fingerprints.printable(c)).collect();
    if missing.is_empty() {
        return false;
    }
    fingerprints.want(missing);
    true
}

/// Both sides' folders' chips in `b` (as the round set them up) and the
/// Program Advances the content makes of them.
pub fn folders_chips(b: &Battle) -> Vec<nettai_content_api::ChipHandle> {
    setup_chips(&b.content, &b.setup)
}

/// The same of a round's setup, `setup` (a set's first, as a host sets a
/// session up: its every round has those folders).
pub fn setup_chips(content: &Content, setup: &nettai_battle::setup::RoundSetup) -> Vec<nettai_content_api::ChipHandle> {
    let mut chips: std::collections::BTreeSet<u16> = setup.players.iter().flat_map(|p| p.folder.chips.iter().flatten().map(|c| c.id.0)).collect();
    for pa in content.program_advances() {
        let made = match &pa.recipe {
            nettai_battle::content::Recipe::CodeRun { chip, .. } => chips.contains(&chip.0),
            nettai_battle::content::Recipe::Sequence(v) => v.iter().all(|c| chips.contains(&c.0)),
        };
        if made {
            chips.insert(pa.result.0);
        }
    }
    chips.into_iter().map(nettai_content_api::ChipHandle).collect()
}

impl Ai for NetAi {
    fn fight(&mut self, b: &Battle, side: u8, _theirs: u16) -> u16 {
        self.tick(b, side)
    }

    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
        self.tick(b, side)
    }

    fn reset(&mut self) {
        self.round = None;
        self.pending = None;
        self.rng = SplitMix64::new(self.seed);
        self.mem.reset();
        self.error = None;
        *nettai_platform::thread::lock(&self.told).unwrap_or_else(|p| p.into_inner()) = None;
    }
}

/// [`NetAi`] on a network that may still be arriving ([`Arriving`]): till it
/// has, a decision waits as one waits for a chip's fingerprint (in the fight
/// it presses nothing, its custom screen waits); then it is that network's
/// `NetAi` at its level. One that arrives as an error, or without the level,
/// stops it, which its host reads ([`WaitingAi::stopped`]).
pub struct WaitingAi {
    net: Arriving,
    level: Level,
    seed: u64,
    fingerprints: Fingerprinter,
    ai: Option<NetAi>,
    error: Option<String>,
    told: Arc<Mutex<Option<String>>>,
}

impl WaitingAi {
    /// The AI of the network `net` will bring, at `level`, its draws from
    /// `seed`, the game's chips' fingerprints from `fingerprints`.
    pub fn new(net: Arriving, level: Level, seed: u64, fingerprints: Fingerprinter) -> WaitingAi {
        WaitingAi { net, level, seed, fingerprints, ai: None, error: None, told: Default::default() }
    }

    /// What its host reads of it once it has given it away: why it stopped,
    /// if it did ([`NetAi::stopped`]; its network's arriving as an error
    /// too).
    pub fn stopped(&self) -> Stopped {
        Stopped { why: self.told.clone(), fingerprints: self.fingerprints.clone() }
    }

    /// Why it stopped playing, if it did.
    pub fn error(&self) -> Option<String> {
        self.ai.as_ref().and_then(NetAi::error).or_else(|| self.error.clone()).or_else(|| self.fingerprints.error())
    }

    /// Its `NetAi`, once its network has arrived (made then).
    fn ai(&mut self) -> Option<&mut NetAi> {
        if self.ai.is_none() && self.error.is_none() {
            let made = match self.net.get()? {
                Ok(net) => NetAi::of(net.clone(), self.level, self.seed, self.fingerprints.clone()),
                Err(why) => Err(why.clone()),
            };
            match made {
                Ok(mut ai) => {
                    ai.told = self.told.clone();
                    self.ai = Some(ai);
                }
                Err(why) => {
                    *nettai_platform::thread::lock(&self.told).unwrap_or_else(|p| p.into_inner()) = Some(why.clone());
                    self.error = Some(why);
                }
            }
        }
        self.ai.as_mut()
    }
}

impl Ai for WaitingAi {
    fn fight(&mut self, b: &Battle, side: u8, theirs: u16) -> u16 {
        self.ai().map_or(0, |ai| ai.fight(b, side, theirs))
    }

    fn custom_screen(&mut self, b: &Battle, side: u8) -> u16 {
        self.ai().map_or(0, |ai| ai.custom_screen(b, side))
    }

    fn reset(&mut self) {
        if let Some(ai) = &mut self.ai {
            ai.reset();
        }
    }
}

#[cfg(test)]
mod tests;
