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
//!   is idle and the screen reads keys, from the screen as it is, the keys
//!   probed in a copy of the battle as the screen first reads them. It holds
//!   a fight action till the next decision; the navigator presses the way to
//!   a slot or a key (act.rs). It never reads `theirs`: it doesn't hear the
//!   player's buttons.
//! - **Its memory.** The GRU state carries through a round and starts over
//!   at the next (a new round is a new score of the set), and when the set
//!   starts over (`reset`), its draws too.
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
//! - **Its weights.** The app's are the shipped file ([`WEIGHTS`]), read
//!   once ([`shipped`]), every `NetAi` sharing the network ([`NetAi::of`]).
//!
//! Nothing here names a game.

use crate::act::{self, Navigator};
use crate::env::{self, fight_runs, probe_keys};
use crate::fingerprint::{self, Fingerprints};
use crate::net::{self, Heads, Knobs, Memory, Net};
use crate::observe::{self, AttackHistory, Extra, Names, Obs};
use crate::spec;
use nettai_battle::battle::mode;
use nettai_battle::setup::SetScore;
use nettai_battle::{Battle, Content};
use nettai_frontend::driver::Ai;
use nettai_netplay::rng::SplitMix64;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// The trained network the app plays with (docs/design/training-ai.md
/// §10.5): the trainer's export (`nettai_ai.export`), the same on the
/// desktop and in a browser, replaced by a better one as it is trained.
pub const WEIGHTS: &[u8] = include_bytes!("../model/nettai-ai.safetensors");

/// The network of [`WEIGHTS`], read once in a process: every [`NetAi`] of
/// the app's shares it ([`NetAi::of`]). Err: why this encoder can't play
/// them (the same each time).
pub fn shipped() -> Result<Arc<Net>, String> {
    static NET: OnceLock<Result<Arc<Net>, String>> = OnceLock::new();
    NET.get_or_init(|| Net::from_safetensors(WEIGHTS).map(Arc::new)).clone()
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
    /// before only OK is allowed: the env's, as it was trained.
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
        let cfg = env::Config::default();
        Ok(NetAi {
            mem: net.memory(),
            net,
            knobs,
            seed,
            rng: SplitMix64::new(seed),
            k: cfg.k,
            screen_decisions: cfg.screen_decisions,
            fingerprints,
            names: None,
            round: None,
            obs: Obs::new(),
            error: None,
            told: Default::default(),
            #[cfg(test)]
            decided: Vec::new(),
        })
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

    fn extra(&self, r: &Round) -> Extra {
        Extra {
            delay: self.knobs.delay,
            log: r.log,
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
            });
            self.mem.reset();
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
        let names = &self.names.as_ref().expect("names").0;
        // The view a later decision will need (the env's ring).
        if r.t > 0 && (r.t + d).is_multiple_of(k) {
            let extra = self.extra(r);
            let at = (((r.t + d) / k) as usize) % r.ring.len();
            let slot = &mut r.ring[at];
            slot.0 = Some(r.t);
            observe::encode(b, side, names, &extra, &mut slot.1);
        }
        // A decision (the env's `asks`, then `ask`).
        let custom = b.round.mode == mode::CUSTOM;
        let screen = b.custom.sides[side as usize].screen;
        if fight_runs(b) && r.t.is_multiple_of(k) && b.player(side).is_some() {
            let want = r.t.checked_sub(d);
            let slot = &r.ring[((r.t / k) as usize) % r.ring.len()];
            let delayed = want.is_some() && slot.0 == want;
            let obs = if delayed {
                &slot.1
            } else {
                // (A decision earlier than the delay: what it sees now.)
                let extra = self.extra(r);
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
                    Some(a) => r.action = a as i32,
                    None => self.stop("a fight decision with nothing allowed".into()),
                }
            }
        } else if custom && r.nav.idle() && screen.is_some_and(|s| act::reads_keys(&s)) {
            if r.probe.is_none() {
                r.probe = Some(probe_keys(b, side));
            }
            let extra = self.extra(r);
            observe::encode(b, side, names, &extra, &mut self.obs);
            // (No decision on a view without a chip's fingerprint: the screen
            // waits, and asks again next tick.)
            if !waits(&self.fingerprints, &self.obs) {
                let s = screen.expect("a screen");
                let legal = act::legal_screen(&s, &b.game_rules().custom_screen, extra.refused, extra.only_ok);
                let out = self.net.forward(&self.obs, &mut self.mem, Heads::Screen);
                let logits = out.screen.expect("the screen's head");
                match net::choose(&logits, &legal, &self.knobs, &mut self.rng) {
                    Some(a) => {
                        r.nav.point(a as i32);
                        r.decisions += 1;
                    }
                    None => self.stop("a custom screen decision with nothing allowed".into()),
                }
            }
        }
        // This tick's buttons (the env's `tick`): a new screen's navigator
        // from nothing; on the screen the navigator's, in the fight the
        // action held.
        if custom && !r.in_screen {
            r.nav.reset();
            r.probe = None;
            r.decisions = 0;
        }
        r.in_screen = custom;
        if self.error.is_some() {
            0
        } else if custom {
            r.nav.buttons(b, side)
        } else if fight_runs(b) {
            act::fight_buttons(r.action)
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
        self.rng = SplitMix64::new(self.seed);
        self.mem.reset();
        self.error = None;
        *nettai_platform::thread::lock(&self.told).unwrap_or_else(|p| p.into_inner()) = None;
    }
}

#[cfg(test)]
mod tests;
