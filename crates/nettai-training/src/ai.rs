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
//! - **Its chips' fingerprints.** Every chip of the game, as the trainer's
//!   pool has them: a chip's conditioned runs put a status on the target with
//!   a chip of the whole game's, so a folder's chips alone come out
//!   otherwise. A [`Fingerprinter`] reads them from the host's cache folder by
//!   content hash, or makes them on threads of its own and writes them
//!   there. Till they are made the AI presses nothing and sees nothing (its
//!   custom screen waits); the network never sees a chip whose fingerprint
//!   isn't made, and the battle's thread never waits for one.
//!
//! Nothing here names a game.

use crate::act::{self, Navigator};
use crate::env::{self, fight_runs, probe_keys};
use crate::fingerprint::{self, Fingerprints, Job};
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

/// Where a game's fingerprints are.
#[derive(Clone, Debug)]
pub enum Status {
    /// Being read from the cache folder.
    Reading,
    /// Being made: items done of all (two a chip).
    Making { done: usize, of: usize },
    Ready(Arc<Fingerprints>),
    Failed(String),
}

/// A game's chips' fingerprints for live play, all of them: read from a
/// cache folder (`fingerprints-<game>-<hash>.json`, as the trainer's pool
/// keeps them), or else made on threads of their own
/// (`nettai_platform::thread::spawn`: the browser's workers too) and written
/// there. One a content in a process: another [`Fingerprinter::start`] of the
/// same content is the same one.
#[derive(Clone)]
pub struct Fingerprinter(Arc<Shared>);

struct Shared {
    hash: u64,
    job: OnceLock<Arc<Job>>,
    made: OnceLock<Result<Arc<Fingerprints>, String>>,
    /// Why the fingerprints made couldn't be kept in the cache folder.
    unkept: OnceLock<String>,
}

impl Fingerprinter {
    /// The fingerprints of `content`'s chips: those this process has (made or
    /// being made), or else read from `cache`, or made on `threads` threads
    /// and written there. The work is on a thread of its own from the start:
    /// this returns at once. A thread that can't be started fails it.
    pub fn start(content: &Arc<Content>, threads: usize, cache: Option<PathBuf>) -> Fingerprinter {
        static STARTED: OnceLock<Mutex<HashMap<u64, Fingerprinter>>> = OnceLock::new();
        let hash = content.hash().0;
        let started = STARTED.get_or_init(Default::default);
        let mut started = nettai_platform::thread::lock(started).unwrap_or_else(|p| p.into_inner());
        if let Some(f) = started.get(&hash) {
            return f.clone();
        }
        let f = Fingerprinter::begin(content, fingerprint::game_chips(content), threads, cache);
        started.insert(hash, f.clone());
        f
    }

    /// The fingerprints of `chips` of `content`, read from `cache` or made on
    /// `threads` threads and written there, on a thread of their own.
    fn begin(content: &Arc<Content>, chips: Vec<nettai_content_api::ChipHandle>, threads: usize, cache: Option<PathBuf>) -> Fingerprinter {
        let f = Fingerprinter(Arc::new(Shared { hash: content.hash().0, job: OnceLock::new(), made: OnceLock::new(), unkept: OnceLock::new() }));
        let shared = f.0.clone();
        let content = content.clone();
        let threads = threads.max(1);
        let spawned = nettai_platform::thread::spawn("fingerprints", move || {
            let file = cache.map(|d| d.join(fingerprint::file_name(&content)));
            let read = file.as_ref().and_then(|f| nettai_platform::fs::read(f).ok()).and_then(|bytes| serde_json::from_slice::<Fingerprints>(&bytes).ok());
            if let Some(f) = read.filter(|f| f.version == fingerprint::VERSION && f.hash == shared.hash) {
                let _ = shared.made.set(Ok(Arc::new(f)));
                return;
            }
            let job = Arc::new(Job::new(&content, &chips));
            let _ = shared.job.set(job.clone());
            for k in 1..threads {
                let (job, worker, file) = (job.clone(), shared.clone(), file.clone());
                if let Err(e) = nettai_platform::thread::spawn(&format!("fingerprints {k}"), move || work(&job, &worker, file.as_deref())) {
                    let _ = shared.made.set(Err(format!("a thread to make the chips' fingerprints on: {e}")));
                    return;
                }
            }
            work(&job, &shared, file.as_deref());
        });
        if let Err(e) = spawned {
            let _ = f.0.made.set(Err(format!("a thread to make the chips' fingerprints on: {e}")));
        }
        f
    }

    /// Fingerprints a host has already: ready at once.
    pub fn made(fingerprints: Arc<Fingerprints>) -> Fingerprinter {
        let made = OnceLock::new();
        let _ = made.set(Ok(fingerprints.clone()));
        Fingerprinter(Arc::new(Shared { hash: fingerprints.hash, job: OnceLock::new(), made, unkept: OnceLock::new() }))
    }

    /// Fingerprints never made (tests: an AI waiting for them).
    #[cfg(test)]
    fn pending(hash: u64) -> Fingerprinter {
        Fingerprinter(Arc::new(Shared { hash, job: OnceLock::new(), made: OnceLock::new(), unkept: OnceLock::new() }))
    }

    /// The fingerprints, once they are read or made.
    pub fn ready(&self) -> Option<&Arc<Fingerprints>> {
        self.0.made.get().and_then(|r| r.as_ref().ok())
    }

    /// The content hash they are of.
    pub fn hash(&self) -> u64 {
        self.0.hash
    }

    pub fn status(&self) -> Status {
        match (self.0.made.get(), self.0.job.get()) {
            (Some(Ok(f)), _) => Status::Ready(f.clone()),
            (Some(Err(e)), _) => Status::Failed(e.clone()),
            (None, Some(job)) => {
                let (done, of) = job.progress();
                Status::Making { done, of }
            }
            (None, None) => Status::Reading,
        }
    }

    /// Why the fingerprints made couldn't be written to the cache folder (they
    /// are made again next time), if they couldn't.
    pub fn unkept(&self) -> Option<&str> {
        self.0.unkept.get().map(String::as_str)
    }
}

/// A fingerprinting thread's work: the job's chips till none is left, then,
/// if they are all done, the fingerprints given out (once) and written to
/// `file`.
fn work(job: &Job, shared: &Shared, file: Option<&std::path::Path>) {
    job.work();
    let Some(result) = job.result() else { return };
    if shared.made.set(result.clone()).is_err() {
        return;
    }
    if let (Ok(f), Some(file)) = (result, file) {
        let kept = serde_json::to_vec(&**f).map_err(|e| e.to_string()).and_then(|bytes| {
            if let Some(dir) = file.parent() {
                nettai_platform::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            nettai_platform::fs::write(file, bytes).map_err(|e| format!("{}: {e}", file.display()))
        });
        if let Err(e) = kept {
            let _ = shared.unkept.set(e);
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
    /// The game's names with its fingerprints, once they are ready.
    names: Option<Names>,
    mem: Memory,
    round: Option<Round>,
    obs: Obs,
    error: Option<String>,
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
            #[cfg(test)]
            decided: Vec::new(),
        })
    }

    /// Whether it plays (its game's fingerprints are made); till then it
    /// presses nothing.
    pub fn ready(&self) -> bool {
        self.fingerprints.ready().is_some()
    }

    /// Its fingerprints' status, for a host to show.
    pub fn fingerprints(&self) -> &Fingerprinter {
        &self.fingerprints
    }

    /// Why it stopped playing, if it did (its fingerprints failed or are of
    /// another content, a decision with nothing allowed): it presses nothing
    /// from then on.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref().or_else(|| match self.fingerprints.0.made.get() {
            Some(Err(e)) => Some(e.as_str()),
            _ => None,
        })
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
        if self.error.is_some() {
            return 0;
        }
        let Some(fps) = self.fingerprints.ready() else { return 0 };
        if self.names.is_none() {
            if fps.hash != b.setup.content.0 {
                self.error = Some(format!("the fingerprints are of content {:016x}, the battle's is {}", fps.hash, b.setup.content));
                return 0;
            }
            self.names = Some(Names::with_fingerprints(&b.content, fps.clone()));
        }
        let names = self.names.as_ref().expect("names");
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
            #[cfg(test)]
            self.decided.push((r.t, if delayed { r.t - d } else { r.t }));
            let obs = if delayed {
                &slot.1
            } else {
                // (A decision earlier than the delay: what it sees now.)
                let extra = self.extra(r);
                observe::encode(b, side, names, &extra, &mut self.obs);
                &self.obs
            };
            let out = self.net.forward(obs, &mut self.mem, Heads::Fight);
            let logits = out.fight.expect("the fight's head");
            match net::choose(&logits, &[true; spec::FIGHT_ACTIONS], &self.knobs, &mut self.rng) {
                Some(a) => r.action = a as i32,
                None => self.error = Some("a fight decision with nothing allowed".into()),
            }
        } else if custom && r.nav.idle() && screen.is_some_and(|s| act::reads_keys(&s)) {
            if r.probe.is_none() {
                r.probe = Some(probe_keys(b, side));
            }
            let extra = self.extra(r);
            observe::encode(b, side, names, &extra, &mut self.obs);
            let s = screen.expect("a screen");
            let legal = act::legal_screen(&s, &b.game_rules().custom_screen, extra.refused, extra.only_ok);
            let out = self.net.forward(&self.obs, &mut self.mem, Heads::Screen);
            let logits = out.screen.expect("the screen's head");
            match net::choose(&logits, &legal, &self.knobs, &mut self.rng) {
                Some(a) => {
                    r.nav.point(a as i32);
                    r.decisions += 1;
                }
                None => self.error = Some("a custom screen decision with nothing allowed".into()),
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
    }
}

#[cfg(test)]
mod tests;
