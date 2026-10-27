//! Many envs on persistent worker threads, stepped as one batch
//! (docs/design/training-ai.md §6.5, §6.6).
//!
//! A worker keeps its thread's content runtimes for its whole life (a Luau
//! VM a game, about a second to load: `behavior`'s per-thread cache), and
//! any worker steps any env (a battle is plain data). [`Pool::step`] hands
//! every worker the batch's actions; each claims the next env not yet
//! stepped, steps it to its next decision and writes its rows of the pool's
//! [`Buffers`], until none is left: a slow core, or an env starting a round,
//! holds up no other's. The buffers are the pool's and never reallocated
//! (the bridge shows them to Python as numpy arrays without a copy). While
//! a worker waits for the next step it makes builds ahead (`Env::prepare`),
//! from each game's own stream of draws, so a round's builds are the same
//! however busy the workers were.

use crate::bots::BotKind;
use crate::env::{self, Config, Controller, Env, Game};
use crate::spec;
use crate::stats::RoundStats;
use nettai_battle::Content;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::Mutex;

/// What a pool is made with (§6.6's `Pool(...)`).
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// The games, by name; each env draws one each round.
    pub games: Vec<String>,
    pub envs: usize,
    pub threads: usize,
    pub seed: u64,
    /// The content directory (none: `nettai_content::index::content`, the
    /// repository's): the battles are its alone (no asset pack is read).
    pub content: Option<PathBuf>,
    pub env: Config,
    /// Where the chips' fingerprints are kept on disk (none: computed once a
    /// process: `Fingerprints::for_game`).
    pub cache: Option<PathBuf>,
    /// Threads that only make builds ahead; with any, the workers that step
    /// make none (a build takes up to 130 ms, a step a millisecond or two:
    /// a worker making one when a step comes leaves it short-handed). None:
    /// the workers make them while they wait.
    pub builders: usize,
}

/// The pool's arrays, refilled by every step (§6.6): by set, the
/// observation's features `[E, 2, C, W]`, ids `[E, 2, C, I]` and masks
/// `[E, 2, C]`; by env and seat, the phase, the actions allowed, the HP
/// gain, the result and the delay; by env, whether the round ended (and
/// was cut off), and its game.
#[derive(Debug)]
pub struct Buffers {
    pub envs: usize,
    pub x: Vec<Vec<f32>>,
    pub ids: Vec<Vec<i32>>,
    pub mask: Vec<Vec<bool>>,
    pub phase: Vec<i8>,
    pub legal_fight: Vec<bool>,
    pub legal_screen: Vec<bool>,
    pub hp_gain: Vec<f32>,
    pub done: Vec<bool>,
    pub truncated: Vec<bool>,
    pub ending: Vec<i8>,
    pub result: Vec<i8>,
    pub game: Vec<i8>,
    pub delay: Vec<i16>,
}

impl Buffers {
    fn new(envs: usize) -> Buffers {
        let seats = envs * 2;
        Buffers {
            envs,
            x: spec::SETS.iter().map(|s| vec![0.0; seats * s.count * s.width]).collect(),
            ids: spec::SETS.iter().map(|s| vec![0; seats * s.count * s.ids]).collect(),
            mask: spec::SETS.iter().map(|s| vec![false; seats * s.count]).collect(),
            phase: vec![0; seats],
            legal_fight: vec![false; seats * spec::FIGHT_ACTIONS],
            legal_screen: vec![false; seats * spec::SCREEN_ACTIONS],
            hp_gain: vec![0.0; seats],
            done: vec![false; envs],
            truncated: vec![false; envs],
            ending: vec![0; envs],
            result: vec![0; seats],
            game: vec![0; envs],
            delay: vec![0; seats],
        }
    }
}

/// Where a worker writes an env's rows: the buffers' memory, which the pool
/// keeps alive and in place while any worker lives. Workers write disjoint
/// rows (the envs each claimed), and only while the pool waits for them in a
/// call that holds it mutably.
#[derive(Clone, Copy)]
struct Rows {
    x: [*mut f32; spec::SETS.len()],
    ids: [*mut i32; spec::SETS.len()],
    mask: [*mut bool; spec::SETS.len()],
    phase: *mut i8,
    legal_fight: *mut bool,
    legal_screen: *mut bool,
    hp_gain: *mut f32,
    done: *mut bool,
    truncated: *mut bool,
    ending: *mut i8,
    result: *mut i8,
    game: *mut i8,
    delay: *mut i16,
}

// SAFETY: the pointers are into the pool's buffers, which outlive the
// workers (the pool joins them before it drops the buffers) and never move
// (they are never resized). A worker writes an env's rows only for an env it
// claimed in the command it runs (each env is claimed once a command), while
// the pool's own thread is blocked waiting for every worker.
unsafe impl Send for Rows {}

impl Rows {
    fn of(b: &mut Buffers) -> Rows {
        Rows {
            x: std::array::from_fn(|i| b.x[i].as_mut_ptr()),
            ids: std::array::from_fn(|i| b.ids[i].as_mut_ptr()),
            mask: std::array::from_fn(|i| b.mask[i].as_mut_ptr()),
            phase: b.phase.as_mut_ptr(),
            legal_fight: b.legal_fight.as_mut_ptr(),
            legal_screen: b.legal_screen.as_mut_ptr(),
            hp_gain: b.hp_gain.as_mut_ptr(),
            done: b.done.as_mut_ptr(),
            truncated: b.truncated.as_mut_ptr(),
            ending: b.ending.as_mut_ptr(),
            result: b.result.as_mut_ptr(),
            game: b.game.as_mut_ptr(),
            delay: b.delay.as_mut_ptr(),
        }
    }

    /// Env `e`'s rows, after a step that ended on `out`.
    ///
    /// SAFETY: `e` is one of the calling worker's envs, and the pool waits.
    unsafe fn write(&self, e: usize, env: &Env, out: &env::Outcome) {
        unsafe {
            for s in 0..2 {
                let at = e * 2 + s;
                *self.phase.add(at) = out.phase[s] as i8;
                *self.hp_gain.add(at) = out.hp_gain[s];
                *self.result.add(at) = out.result[s];
                *self.delay.add(at) = env.delay()[s] as i16;
                if out.phase[s] == env::NONE {
                    continue;
                }
                for (k, set) in spec::SETS.iter().enumerate() {
                    let o = &env.obs[s].sets[k];
                    std::ptr::copy_nonoverlapping(o.x.as_ptr(), self.x[k].add(at * set.count * set.width), o.x.len());
                    std::ptr::copy_nonoverlapping(o.ids.as_ptr(), self.ids[k].add(at * set.count * set.ids), o.ids.len());
                    std::ptr::copy_nonoverlapping(o.mask.as_ptr(), self.mask[k].add(at * set.count), o.mask.len());
                }
                std::ptr::copy_nonoverlapping(env.legal_fight[s].as_ptr(), self.legal_fight.add(at * spec::FIGHT_ACTIONS), spec::FIGHT_ACTIONS);
                std::ptr::copy_nonoverlapping(env.legal_screen[s].as_ptr(), self.legal_screen.add(at * spec::SCREEN_ACTIONS), spec::SCREEN_ACTIONS);
            }
            *self.done.add(e) = out.done;
            *self.truncated.add(e) = out.truncated;
            *self.ending.add(e) = out.ending as i8;
            *self.game.add(e) = env.game() as i8;
        }
    }
}

/// The envs, shared by the workers: each step, each worker claims the next
/// env not yet stepped until none is left, so a slow core or a round's
/// start holds up no other worker's envs.
struct Shared {
    envs: Vec<Mutex<Option<Env>>>,
    /// Each env's builds, as it is made: a worker makes builds ahead there
    /// without the env's lock.
    builds: Vec<std::sync::OnceLock<Arc<env::EnvBuilds>>>,
    next: AtomicUsize,
    /// The builders end.
    stop: std::sync::atomic::AtomicBool,
}

impl Shared {
    fn env(&self, e: usize) -> std::sync::MutexGuard<'_, Option<Env>> {
        // (A worker that panicked holding one stopped the pool: its message
        // is above; the env is as it was.)
        self.envs[e].lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// What the pool asks of its workers.
enum Cmd {
    /// Make the envs (each its runtimes loaded first).
    Start,
    Step(Arc<Vec<i32>>),
}

struct Worker {
    tx: Sender<Cmd>,
    rx: Receiver<Result<(), String>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// A pair of chips as words: the setup's key, the payoff's, the condition,
/// the ratio.
pub type PairNamed = (String, String, String, f32);

/// N envs on persistent worker threads.
pub struct Pool {
    games: Vec<String>,
    /// What the envs were made with.
    config: Config,
    made: Arc<Vec<Game>>,
    workers: Vec<Worker>,
    builders: Vec<std::thread::JoinHandle<()>>,
    shared: Arc<Shared>,
    buffers: Buffers,
}

/// Game `game`'s battle content, from the content directory `content`
/// (none: the repository's): the content alone, as a battle reads it (its
/// sprites' timing too: no asset pack, picture or sound).
pub fn load_game(content: Option<&Path>, game: &str) -> Result<Arc<Content>, String> {
    let dir = content.map(Path::to_path_buf).unwrap_or_else(nettai_content::index::content);
    let (content, _) = nettai_content::load::load_battle(&dir, game).map_err(|r| format!("{game}: {r}"))?;
    Ok(Arc::new(content))
}

impl Pool {
    /// A pool as `cfg` says, its games loaded from the content.
    pub fn new(cfg: PoolConfig) -> Result<Pool, String> {
        let contents = cfg.games.iter().map(|g| load_game(cfg.content.as_deref(), g)).collect::<Result<Vec<_>, _>>()?;
        Pool::with_contents(cfg, contents)
    }

    /// A pool on contents already loaded (by game, in `cfg.games`' order),
    /// with their chips' fingerprints (`Fingerprints::for_game`, on the
    /// pool's threads).
    pub fn with_contents(cfg: PoolConfig, contents: Vec<Arc<Content>>) -> Result<Pool, String> {
        env::quiet_engine_panics();
        let games = contents
            .into_iter()
            .map(|c| {
                let f = crate::fingerprint::Fingerprints::for_game(&c, cfg.threads, cfg.cache.as_deref())?;
                Game::with_fingerprints(c, f)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Pool::with_games(cfg, Arc::new(games))
    }

    /// A pool on games already made (by game, in `cfg.games`' order).
    pub fn with_games(cfg: PoolConfig, games: Arc<Vec<Game>>) -> Result<Pool, String> {
        if cfg.envs == 0 || cfg.threads == 0 {
            return Err("a pool has at least one env and one thread".into());
        }
        if cfg.games.len() > i8::MAX as usize {
            return Err("too many games".into());
        }
        env::quiet_engine_panics();
        for (name, g) in cfg.games.iter().zip(games.iter()) {
            if *name != g.name {
                return Err(format!("the content loaded for {name} is {}'s", g.name));
            }
        }
        let mut buffers = Buffers::new(cfg.envs);
        let rows = Rows::of(&mut buffers);
        let shared = Arc::new(Shared {
            envs: (0..cfg.envs).map(|_| Mutex::new(None)).collect(),
            builds: (0..cfg.envs).map(|_| Default::default()).collect(),
            next: AtomicUsize::new(0),
            stop: Default::default(),
        });
        let mut workers = Vec::with_capacity(cfg.threads);
        let prepares = cfg.builders == 0;
        for w in 0..cfg.threads {
            let (tx, rx_cmd) = channel();
            let (tx_reply, rx) = channel();
            let (games, config, seed, shared) = (games.clone(), cfg.env.clone(), cfg.seed, shared.clone());
            let thread = std::thread::Builder::new()
                .name(format!("nettai-pool-{w}"))
                .spawn(move || work(games, config, seed, shared, rows, rx_cmd, tx_reply, prepares))
                .map_err(|e| format!("a worker thread: {e}"))?;
            workers.push(Worker { tx, rx, thread: Some(thread) });
        }
        let mut builders = Vec::with_capacity(cfg.builders);
        for b in 0..cfg.builders {
            let (games, shared) = (games.clone(), shared.clone());
            let thread = std::thread::Builder::new()
                .name(format!("nettai-builder-{b}"))
                .spawn(move || build_ahead(games, shared))
                .map_err(|e| format!("a builder thread: {e}"))?;
            builders.push(thread);
        }
        let mut pool = Pool { games: cfg.games.clone(), config: cfg.env.clone(), made: games, workers, builders, shared, buffers };
        // (The envs made, each at its first decision.)
        pool.run(|| Cmd::Start)?;
        Ok(pool)
    }

    pub fn games(&self) -> &[String] {
        &self.games
    }

    /// What the envs were made with (their `k`, their screen decisions...).
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Each game's pairs of chips (setup key, payoff key, the condition as
    /// words, the ratio).
    pub fn pairs(&self) -> Vec<(String, Vec<PairNamed>)> {
        self.made
            .iter()
            .map(|g| {
                let c = &g.content;
                let pairs = g.names.fingerprints().map_or_else(Vec::new, |f| {
                    f.pairs
                        .iter()
                        .map(|p| {
                            let condition = match p.condition {
                                crate::fingerprint::Condition::Kind(k) => format!("panel {}", c.rules().panels.names.get(k.0 as usize).map_or("?", |n| n.as_str())),
                                crate::fingerprint::Condition::Status(s) => format!("status {s}"),
                                crate::fingerprint::Condition::Advanced => "front line advanced".to_string(),
                                crate::fingerprint::Condition::Stolen => "front column stolen".to_string(),
                            };
                            (c.defs.chip(p.setup).key.clone(), c.defs.chip(p.payoff).key.clone(), condition, p.ratio)
                        })
                        .collect()
                });
                (g.name.clone(), pairs)
            })
            .collect()
    }

    pub fn envs(&self) -> usize {
        self.buffers.envs
    }

    pub fn buffers(&self) -> &Buffers {
        &self.buffers
    }

    /// The buffers' memory, to show without a copy (the bridge's numpy
    /// views). It stays where it is for the pool's life.
    pub fn buffers_mut(&mut self) -> &mut Buffers {
        &mut self.buffers
    }

    /// Every worker on `cmd` over the envs, until each env is done; the
    /// first error, if any.
    fn run(&mut self, cmd: impl Fn() -> Cmd) -> Result<(), String> {
        self.shared.next.store(0, Ordering::SeqCst);
        for (i, w) in self.workers.iter().enumerate() {
            w.tx.send(cmd()).map_err(|_| format!("worker {i} has stopped"))?;
        }
        let mut first = Ok(());
        for (i, w) in self.workers.iter().enumerate() {
            let r = w.rx.recv().map_err(|_| format!("worker {i} stopped (a panic: see its message above)")).and_then(|r| r);
            if first.is_ok() {
                first = r;
            }
        }
        first
    }

    /// Apply `actions` (`[E, 2]`, read where the last step asked a seat),
    /// and step every env to its next decision (§6.6). An action that isn't
    /// allowed refuses the whole step, before any env moves.
    pub fn step(&mut self, actions: &[i32]) -> Result<(), String> {
        if actions.len() != self.envs() * 2 {
            return Err(format!("{} actions for {} envs (2 a env)", actions.len(), self.envs()));
        }
        let b = &self.buffers;
        for (at, &a) in actions.iter().enumerate() {
            let legal = match b.phase[at] {
                1 => usize::try_from(a).ok().and_then(|a| (a < spec::FIGHT_ACTIONS).then(|| b.legal_fight[at * spec::FIGHT_ACTIONS + a])),
                2 => usize::try_from(a).ok().and_then(|a| (a < spec::SCREEN_ACTIONS).then(|| b.legal_screen[at * spec::SCREEN_ACTIONS + a])),
                _ => Some(true),
            };
            if legal != Some(true) {
                return Err(format!("env {} seat {}: action {a} isn't allowed (phase {})", at / 2, at % 2, b.phase[at]));
            }
        }
        let actions = Arc::new(actions.to_vec());
        self.run(|| Cmd::Step(actions.clone()))
    }

    /// Each of envs `envs` (all: none given), changed by `f`.
    fn each(&self, envs: Option<&[usize]>, f: impl Fn(&mut Env)) -> Result<(), String> {
        let all: Vec<usize>;
        let list = match envs {
            Some(l) => l,
            None => {
                all = (0..self.envs()).collect();
                &all
            }
        };
        if let Some(e) = list.iter().find(|&&e| e >= self.envs()) {
            return Err(format!("env {e}: the pool has {}", self.envs()));
        }
        for &e in list {
            if let Some(env) = self.shared.env(e).as_mut() {
                f(env);
            }
        }
        Ok(())
    }

    /// Who plays seat `seat` of envs `envs` from their next round.
    pub fn set_seat(&mut self, envs: &[usize], seat: usize, controller: Controller) -> Result<(), String> {
        check_seat(seat)?;
        self.each(Some(envs), |env| env.set_seat(seat, controller))
    }

    /// The delay seat `seat` of envs `envs` plays at from their next round
    /// (none: drawn again).
    pub fn set_delay(&mut self, envs: &[usize], seat: usize, ticks: Option<u16>) -> Result<(), String> {
        check_seat(seat)?;
        self.each(Some(envs), |env| env.set_delay(seat, ticks))
    }

    /// Envs `envs` dealt the same rounds from `key` from their next round
    /// (none: each from its own draws), [`Env::set_deal`].
    pub fn set_deal(&mut self, envs: &[usize], key: Option<u64>) -> Result<(), String> {
        self.each(Some(envs), |env| env.set_deal(key))
    }

    /// The ticks seat `seat` of envs `envs` holds a fight decision from
    /// their next round (none: the env's k).
    pub fn set_k(&mut self, envs: &[usize], seat: usize, k: Option<u32>) -> Result<(), String> {
        check_seat(seat)?;
        self.each(Some(envs), |env| env.set_k(seat, k))
    }

    /// Both sides' base HP drawn from `lo` to `hi` (none: each side's as
    /// its build has it), from the next rounds.
    pub fn set_hp(&mut self, hp: Option<(u16, u16)>) -> Result<(), String> {
        if let Some((lo, hi)) = hp
            && (lo == 0 || lo > hi)
        {
            return Err(format!("HP {lo} to {hi}"));
        }
        self.each(None, |env| env.set_hp(hp))
    }

    /// The chips held out of training, from the next rounds (§6.6).
    pub fn set_chip_holdout(&mut self, holdout: crate::builds::Holdout) -> Result<(), String> {
        if !(0.0..=1.0).contains(&holdout.fraction) {
            return Err(format!("a fraction held out of {}: 0 to 1", holdout.fraction));
        }
        self.each(None, |env| env.set_chip_holdout(holdout))
    }

    /// The rounds ended since the last call.
    pub fn round_stats(&mut self) -> Result<Vec<RoundStats>, String> {
        let mut out = Vec::new();
        for e in 0..self.envs() {
            if let Some(env) = self.shared.env(e).as_mut() {
                out.extend(env.ended.drain(..).map(|r| RoundStats { env: e, ..r }));
            }
        }
        Ok(out)
    }

    /// Where the envs' time went, summed.
    pub fn profile(&self) -> env::Profile {
        let mut p = env::Profile::default();
        for e in 0..self.envs() {
            if let Some(env) = self.shared.env(e).as_ref() {
                let q = env.profile();
                p.encode += q.encode;
                p.reset += q.reset;
                p.probe += q.probe;
                p.prepare += q.prepare;
                p.ticks += q.ticks;
                p.decisions += q.decisions;
                p.rounds += q.rounds;
            }
        }
        p
    }
}

fn check_seat(seat: usize) -> Result<(), String> {
    if seat > 1 { Err(format!("seat {seat}: a seat is 0 or 1")) } else { Ok(()) }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        for b in self.builders.drain(..) {
            b.thread().unpark();
            let _ = b.join();
        }
        for w in &mut self.workers {
            // (A worker whose channel closes ends.)
            let (tx, _) = channel();
            drop(std::mem::replace(&mut w.tx, tx));
        }
        for w in &mut self.workers {
            if let Some(t) = w.thread.take() {
                let _ = t.join();
            }
        }
    }
}

/// A controller by its name (§6.6: "policy" or a scripted player's).
pub fn controller(name: &str) -> Result<Controller, String> {
    Controller::named(name).ok_or_else(|| {
        let bots: Vec<&str> = BotKind::ALL.iter().map(|k| k.name()).collect();
        format!("no controller {name:?} (policy, {})", bots.join(", "))
    })
}

/// A worker's life: its runtimes loaded; then each command, over the envs
/// it claims; and while none waits, builds made ahead for an env no one
/// holds.
#[allow(clippy::too_many_arguments)]
fn work(games: Arc<Vec<Game>>, cfg: Config, seed: u64, shared: Arc<Shared>, rows: Rows, rx: Receiver<Cmd>, tx: Sender<Result<(), String>>, prepares: bool) {
    let loaded: Result<(), String> = games.iter().try_for_each(|g| {
        nettai_battle::behavior::Behaviors::for_content(&g.content).map(|_| ()).map_err(|e| format!("{}: {e}", g.name))
    });
    let mut idle_at = 0;
    loop {
        let cmd = match rx.try_recv() {
            Ok(c) => c,
            Err(TryRecvError::Disconnected) => return,
            Err(TryRecvError::Empty) => {
                // (Nothing asked: a build made ahead, then look again.)
                if prepares && loaded.is_ok() && prepare_one(&shared, &mut idle_at) {
                    continue;
                }
                match rx.recv() {
                    Ok(c) => c,
                    Err(_) => return,
                }
            }
        };
        let result = loaded.clone().and_then(|()| {
            let n = shared.envs.len();
            loop {
                let e = shared.next.fetch_add(1, Ordering::SeqCst);
                if e >= n {
                    return Ok(());
                }
                let mut slot = shared.env(e);
                let out = match &cmd {
                    Cmd::Start => {
                        let mut env = Env::new(games.clone(), cfg.clone(), seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(e as u64)).map_err(|why| format!("env {e}: {why}"))?;
                        let out = env.step([0, 0]).map_err(|why| format!("env {e}: {why}"))?;
                        let _ = shared.builds[e].set(env.builds());
                        *slot = Some(env);
                        out
                    }
                    Cmd::Step(actions) => {
                        let env = slot.as_mut().ok_or_else(|| format!("env {e} wasn't made"))?;
                        env.step([actions[e * 2], actions[e * 2 + 1]]).map_err(|why| format!("env {e}: {why}"))?
                    }
                };
                // SAFETY: env `e` was claimed by this worker alone this round of
                // the command, and the pool waits for every worker's reply.
                unsafe { rows.write(e, slot.as_ref().expect("the env"), &out) };
            }
        });
        if tx.send(result).is_err() {
            return;
        }
    }
}

/// A builder's life: its runtimes loaded; then builds made ahead for the
/// envs as they need them, a short sleep when none does, until the pool
/// ends.
fn build_ahead(games: Arc<Vec<Game>>, shared: Arc<Shared>) {
    let loaded = games.iter().all(|g| nettai_battle::behavior::Behaviors::for_content(&g.content).is_ok());
    let mut at = 0;
    while loaded && !shared.stop.load(Ordering::SeqCst) {
        if !prepare_one(&shared, &mut at) {
            std::thread::park_timeout(std::time::Duration::from_millis(5));
        }
    }
}

/// A build made ahead for one env that needs one and that no one holds,
/// looking from `at` on; whether one was made.
fn prepare_one(shared: &Shared, at: &mut usize) -> bool {
    let n = shared.envs.len();
    for k in 0..n {
        let e = (*at + k) % n;
        let Some(builds) = shared.builds[e].get() else { continue };
        // (An error making it ahead is met again as the round needs it.)
        if let Ok(true) = builds.make_ahead() {
            *at = e;
            return true;
        }
    }
    *at = 0;
    false
}

#[cfg(test)]
mod tests;
